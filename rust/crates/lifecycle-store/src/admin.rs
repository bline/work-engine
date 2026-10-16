//! Offline administration over the sole lifecycle store. These values are
//! observations and exact CAS requests; none grants a new execution entry.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use ed25519_dalek::{Signature, VerifyingKey};
use lifecycle_core::HandoffId;
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Serialize};
use work_engine_types::CodecContract;

use crate::{SqliteLifecycleStore, StoreError};

// Offline custody capsules retain their actual schema across read-only inspection.
// Writable migration remains owned by SqliteLifecycleStore::open_inner.
fn supported_admin_schema(version: i64) -> bool {
    matches!(version, 4..=6)
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
type HeldMarkerRow = (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    i64,
    i64,
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

/// Raw historical closure. Its fields are data, never source authority.
#[derive(Clone, Debug)]
pub struct HeldEvidence {
    pub artifacts: Vec<HeldArtifactInput>,
}

#[derive(Clone, Debug)]
pub struct HeldArtifactInput {
    pub name: String,
    pub raw_bytes: Vec<u8>,
}

/// No production constructor exists until the permanent source verifier joins.
/// A data file, trait implementation or deserializer cannot mint this value.
///
/// ```compile_fail
/// use lifecycle_store::HeldSourceWitness;
/// let _forged = HeldSourceWitness {
///     handoff_id: "self-authored".into(),
///     source_capture_sha256: "self-authored".into(),
///     capsule_sha256: "self-authored".into(),
///     _sealed: (),
/// };
/// ```
///
/// ```compile_fail
/// let _: lifecycle_store::HeldSourceWitness = serde_json::from_str("{}").unwrap();
/// ```
pub struct HeldSourceWitness {
    handoff_id: String,
    source_capture_sha256: String,
    capsule_sha256: String,
    _sealed: (),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HeldProjection {
    pub store_id: String,
    pub ownership_status: String,
    pub handoff_id: String,
    pub source_capture_sha256: String,
    pub capsule_sha256: String,
    pub inventory_sha256: String,
    pub row_count: u64,
    pub unclassified_count: u64,
    pub custody_revision: u64,
    pub artifact_digests: Vec<(String, String)>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HeldCapsuleReceipt {
    pub format: String,
    pub capsule_root: PathBuf,
    pub source_store_id: String,
    pub source_revision: u64,
    pub source_custody_phase: String,
    pub database_sha256: String,
    pub handoff_id: String,
    pub source_capture_sha256: String,
    pub origin_capsule_sha256: String,
    pub inventory_sha256: String,
    pub row_count: u64,
    pub unclassified_count: u64,
    pub artifact_digests: Vec<(String, String)>,
    pub receipt_sha256: String,
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
        if held_quarantine(&self.connection)? {
            return Err(StoreError::Rejected);
        }
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
        if version == 6 && held_quarantine(&inspection)? {
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
        if held_quarantine(&self.connection)? {
            return Err(StoreError::Rejected);
        }
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
    Ok(status == "active" && !held_quarantine(connection)?)
}

pub(crate) fn held_quarantine(connection: &rusqlite::Connection) -> Result<bool, StoreError> {
    let mode: String = connection
        .query_row(
            "SELECT mode FROM legacy_held_state WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .map_err(|_| StoreError::Unavailable)?;
    match mode.as_str() {
        "ordinary" => Ok(false),
        "held_quarantine" => Ok(true),
        _ => Err(StoreError::Unavailable),
    }
}

const HELD_ARTIFACT_NAMES: [&str; 16] = [
    "source/attempt-receipt.json",
    "source/capture/source-manifest.json",
    "source/capture/receipt.json",
    "source/capture/image/semantic-context.sqlite3",
    "source/capture/image/role-bindings.json",
    "source/capture/image/generation-state.json",
    "source/capture/image/switchboard-attachment.json",
    "output/capsule.json",
    "output/inventory-private.json",
    "output/report-redacted.json",
    "output/manifest.json",
    "output/receipt.json",
    "output/raw/q3-disposition.json",
    "output/raw/q3-input.json",
    "output/raw/q4-disposition.json",
    "output/raw/q4-input.json",
];

type HeldRow = (String, String, Option<String>, String, String, Vec<u8>);
type ValidatedHeld = (
    Vec<(String, String, Vec<u8>)>,
    Vec<HeldRow>,
    String,
    String,
    String,
    u64,
);

// The A closure refuses duplicate object keys before any lossy Value parse.
// Recheck that byte-level property where the held store consumes copied bytes.
struct HeldUniqueJson(serde_json::Value);

impl<'de> Deserialize<'de> for HeldUniqueJson {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;
        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = HeldUniqueJson;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("JSON without duplicate object keys")
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(HeldUniqueJson(serde_json::Value::Bool(value)))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(HeldUniqueJson(serde_json::Value::Number(value.into())))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(HeldUniqueJson(serde_json::Value::Number(value.into())))
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(|n| HeldUniqueJson(serde_json::Value::Number(n)))
                    .ok_or_else(|| E::custom("non-finite JSON number"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(HeldUniqueJson(serde_json::Value::String(value.into())))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(HeldUniqueJson(serde_json::Value::String(value)))
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(HeldUniqueJson(serde_json::Value::Null))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(HeldUniqueJson(serde_json::Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = sequence.next_element::<HeldUniqueJson>()? {
                    values.push(value.0);
                }
                Ok(HeldUniqueJson(serde_json::Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some((key, value)) = map.next_entry::<String, HeldUniqueJson>()? {
                    if values.insert(key, value.0).is_some() {
                        return Err(de::Error::custom("duplicate JSON key"));
                    }
                }
                Ok(HeldUniqueJson(serde_json::Value::Object(values)))
            }
        }
        deserializer.deserialize_any(UniqueVisitor)
    }
}

fn held_json(bytes: &[u8]) -> Result<serde_json::Value, StoreError> {
    std::str::from_utf8(bytes).map_err(|_| StoreError::Rejected)?;
    serde_json::from_slice::<HeldUniqueJson>(bytes)
        .map(|v| v.0)
        .map_err(|_| StoreError::Rejected)
}

fn validate_held_evidence(evidence: &HeldEvidence) -> Result<ValidatedHeld, StoreError> {
    if evidence.artifacts.len() != HELD_ARTIFACT_NAMES.len() {
        return Err(StoreError::Rejected);
    }
    let mut artifacts = std::collections::BTreeMap::new();
    for artifact in &evidence.artifacts {
        if !HELD_ARTIFACT_NAMES.contains(&artifact.name.as_str())
            || artifacts
                .insert(artifact.name.as_str(), artifact.raw_bytes.as_slice())
                .is_some()
        {
            return Err(StoreError::Rejected);
        }
    }
    let raw = |name: &str| artifacts.get(name).copied().ok_or(StoreError::Rejected);
    let source_manifest = held_json(raw("source/capture/source-manifest.json")?)?;
    let capture_receipt = held_json(raw("source/capture/receipt.json")?)?;
    let attempt_receipt = held_json(raw("source/attempt-receipt.json")?)?;
    let source_files = source_manifest["files"]
        .as_array()
        .ok_or(StoreError::Rejected)?;
    if source_manifest["schema_version"] != 1
        || source_manifest["selection_claim"] != "current_selection_unestablished"
        || capture_receipt["schema_version"] != 1
        || capture_receipt["status"] != "qualified_candidate_root"
        || capture_receipt["selection_claim"] != "current_selection_unestablished"
        || capture_receipt["manifest_sha256"].as_str()
            != Some(digest(raw("source/capture/source-manifest.json")?)?.as_str())
        || attempt_receipt["schema_version"] != 1
        || attempt_receipt["status"] != "qualified_candidate_root"
        || attempt_receipt["selection_claim"] != "current_selection_unestablished"
        || attempt_receipt["child_manifest_sha256"].as_str()
            != Some(digest(raw("source/capture/source-manifest.json")?)?.as_str())
        || attempt_receipt["child_receipt"]["sha256"].as_str()
            != Some(digest(raw("source/capture/receipt.json")?)?.as_str())
        || source_files.len() != 4
    {
        return Err(StoreError::Rejected);
    }
    let mut image_hashes = std::collections::BTreeMap::new();
    let mut capture_root: Option<PathBuf> = None;
    for name in [
        "semantic-context.sqlite3",
        "role-bindings.json",
        "generation-state.json",
        "switchboard-attachment.json",
    ] {
        let source_name = format!("source/capture/image/{name}");
        let bytes = raw(&source_name)?;
        let hash = digest(bytes)?;
        let manifest_entry = source_files
            .iter()
            .find(|entry| {
                entry["copy"]
                    .as_str()
                    .is_some_and(|copy| Path::new(copy).ends_with(Path::new("image").join(name)))
            })
            .ok_or(StoreError::Rejected)?;
        let copy = Path::new(
            manifest_entry["copy"]
                .as_str()
                .ok_or(StoreError::Rejected)?,
        );
        let root = copy
            .parent()
            .and_then(Path::parent)
            .ok_or(StoreError::Rejected)?;
        if !copy.is_absolute()
            || copy != root.join("image").join(name)
            || capture_root.as_ref().is_some_and(|prior| prior != root)
        {
            return Err(StoreError::Rejected);
        }
        capture_root = Some(root.to_path_buf());
        if manifest_entry["sha256"].as_str() != Some(hash.as_str())
            || manifest_entry["copy_sha256"].as_str() != Some(hash.as_str())
        {
            return Err(StoreError::Rejected);
        }
        let attempt_name = format!("capture/image/{name}");
        let attempt_entry = &attempt_receipt["artifacts"][&attempt_name];
        if attempt_entry["sha256"].as_str() != Some(hash.as_str())
            || attempt_entry["bytes"]
                .as_u64()
                .is_some_and(|length| length != bytes.len() as u64)
            || attempt_entry["path"]
                .as_str()
                .is_some_and(|path| Some(path) != copy.to_str())
        {
            return Err(StoreError::Rejected);
        }
        if name != "semantic-context.sqlite3" {
            let _ = held_json(bytes)?;
        } else if bytes.len() < 100
            || &bytes[..16] != b"SQLite format 3\0"
            || u32::from_be_bytes(bytes[60..64].try_into().map_err(|_| StoreError::Rejected)?) != 12
        {
            return Err(StoreError::Rejected);
        }
        image_hashes.insert(name.to_owned(), hash);
    }
    let capture_root = capture_root.ok_or(StoreError::Rejected)?;
    if capture_receipt["output_root"]
        .as_str()
        .is_some_and(|path| path != capture_root.to_str().unwrap_or(""))
    {
        return Err(StoreError::Rejected);
    }
    for (name, source_name) in [
        (
            "capture/source-manifest.json",
            "source/capture/source-manifest.json",
        ),
        ("capture/receipt.json", "source/capture/receipt.json"),
    ] {
        let bytes = raw(source_name)?;
        if let Some(entry) = attempt_receipt["artifacts"].get(name)
            && (entry["sha256"].as_str() != Some(digest(bytes)?.as_str())
                || entry["bytes"].as_u64() != Some(bytes.len() as u64)
                || entry["path"].as_str()
                    != capture_root
                        .join(name.strip_prefix("capture/").ok_or(StoreError::Rejected)?)
                        .to_str())
        {
            return Err(StoreError::Rejected);
        }
    }
    let output_manifest = held_json(raw("output/manifest.json")?)?;
    let listed_files = output_manifest["files"]
        .as_array()
        .ok_or(StoreError::Rejected)?;
    if output_manifest["schema_version"] != 1 || listed_files.len() != 7 {
        return Err(StoreError::Rejected);
    }
    let mut listed_names = std::collections::BTreeSet::new();
    for entry in listed_files {
        let path = entry["path"].as_str().ok_or(StoreError::Rejected)?;
        let name = format!("output/{path}");
        if path == "manifest.json"
            || path == "receipt.json"
            || !listed_names.insert(name.clone())
            || entry["sha256"].as_str() != Some(digest(raw(&name)?)?.as_str())
            || entry["length"].as_u64() != Some(raw(&name)?.len() as u64)
        {
            return Err(StoreError::Rejected);
        }
    }
    let expected_listed: std::collections::BTreeSet<_> = HELD_ARTIFACT_NAMES
        .iter()
        .filter(|name| {
            name.starts_with("output/")
                && **name != "output/manifest.json"
                && **name != "output/receipt.json"
        })
        .map(|name| (*name).to_owned())
        .collect();
    if listed_names != expected_listed {
        return Err(StoreError::Rejected);
    }
    let output_receipt = held_json(raw("output/receipt.json")?)?;
    if output_receipt["schema_version"] != 1
        || output_receipt["capsule_version"] != "s9-v28-obligations-v2"
        || output_receipt["status"] != "qualified_translation_only"
        || output_receipt["transfer_eligibility"] != "blocked_unclassified"
        || output_receipt["source_attempt_sha256"].as_str()
            != Some(digest(raw("source/attempt-receipt.json")?)?.as_str())
        || output_receipt["source_manifest_sha256"].as_str()
            != Some(digest(raw("source/capture/source-manifest.json")?)?.as_str())
        || output_receipt["source_child_receipt_sha256"].as_str()
            != Some(digest(raw("source/capture/receipt.json")?)?.as_str())
        || output_receipt["manifest_sha256"].as_str()
            != Some(digest(raw("output/manifest.json")?)?.as_str())
        || output_receipt["capsule_sha256"].as_str()
            != Some(digest(raw("output/capsule.json")?)?.as_str())
    {
        return Err(StoreError::Rejected);
    }
    let capsule_raw = raw("output/capsule.json")?;
    let inventory_raw = raw("output/inventory-private.json")?;
    let capsule = held_json(capsule_raw)?;
    let inventory = held_json(inventory_raw)?;
    if capsule["schema_version"] != "s9-v28-obligations-v2"
        || capsule["translation_status"] != "qualified"
        || capsule["transfer_eligibility"] != "blocked_unclassified"
        || capsule["selection_claim"] != "current_selection_unestablished"
        || capsule["source_attempt_sha256"].as_str()
            != Some(digest(raw("source/attempt-receipt.json")?)?.as_str())
        || capsule["source_child_receipt_sha256"].as_str()
            != Some(digest(raw("source/capture/receipt.json")?)?.as_str())
        || capsule["source_manifest_sha256"].as_str()
            != Some(digest(raw("source/capture/source-manifest.json")?)?.as_str())
        || capsule["copied_image_sha256"] != serde_json::json!(image_hashes)
        || capsule["relation_census"].as_array().map(Vec::len) != Some(28)
        || capsule["q3"]["physical_delivery"] != "not_applicable"
        || capsule["q4"]["physical_delivery"] != "not_proved"
    {
        return Err(StoreError::Rejected);
    }
    for label in ["q3", "q4"] {
        let disposition = held_json(raw(&format!("output/raw/{label}-disposition.json"))?)?;
        let input_raw = raw(&format!("output/raw/{label}-input.json"))?;
        let input = held_json(input_raw)?;
        let projection = &capsule[label];
        let target = &disposition["target"];
        if projection["queue_id"] != target["queueId"]
            || projection["input_revision"] != input["inputRevision"]
            || projection["disposition_revision"] != disposition["recordRevision"]
            || projection["authority_revision"] != disposition["authorityRevision"]
            || target["inputRevision"] != input["inputRevision"]
            || target["logicalRoleInstanceId"] != input["logicalRoleInstanceId"]
            || target["clientUserMessageId"] != input["clientUserMessageId"]
            || target["bindingRevision"] != input["bindingRevision"]
            || target["inputJsonSha256"].as_str() != Some(digest(input_raw)?.as_str())
        {
            return Err(StoreError::Rejected);
        }
        let successor = &disposition["successorAcceptance"];
        if label == "q3" {
            if !successor.is_null() || !projection["successor_acceptance_revision"].is_null() {
                return Err(StoreError::Rejected);
            }
        } else if successor["acceptanceRevision"] != projection["successor_acceptance_revision"]
            || successor["successorLogicalRoleInstanceId"]
                != projection["successor_logical_role_instance_id"]
            || successor["successorDeliveryIdentity"] != projection["successor_delivery_identity"]
        {
            return Err(StoreError::Rejected);
        }
    }
    let rows = inventory.as_array().ok_or(StoreError::Rejected)?;
    let classification_digests = capsule["row_classification_digests"]
        .as_array()
        .ok_or(StoreError::Rejected)?;
    if rows.len() != classification_digests.len() || rows.is_empty() {
        return Err(StoreError::Rejected);
    }
    let inventory_canonical =
        serde_json::to_string(&inventory).map_err(|_| StoreError::Rejected)?;
    let inventory_sha = digest(inventory_canonical.as_bytes())?;
    if capsule["inventory_digest_sha256"].as_str() != Some(inventory_sha.as_str()) {
        return Err(StoreError::Rejected);
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut stored_rows = Vec::with_capacity(rows.len());
    let mut unclassified = 0u64;
    let mut retired = 0u64;
    let mut pending = 0u64;
    for (row, listed) in rows.iter().zip(classification_digests) {
        let table = row["table"]
            .as_str()
            .filter(|x| !x.is_empty())
            .ok_or(StoreError::Rejected)?;
        let identity = row["identity"]
            .as_str()
            .filter(|x| !x.is_empty())
            .ok_or(StoreError::Rejected)?;
        let revision = row["revision"].as_str().map(str::to_owned);
        let projection = row["source_projection_sha256"]
            .as_str()
            .filter(|x| hex_digest(x))
            .ok_or(StoreError::Rejected)?;
        let class = row["classification"]
            .as_str()
            .filter(|x| !x.is_empty())
            .ok_or(StoreError::Rejected)?;
        let key = (
            table.to_owned(),
            identity.to_owned(),
            revision.clone(),
            projection.to_owned(),
        );
        if !seen.insert(key) {
            return Err(StoreError::Rejected);
        }
        if listed["table"] != table
            || listed["identity_sha256"].as_str() != Some(digest(identity.as_bytes())?.as_str())
            || listed["source_projection_sha256"] != projection
            || listed["classification"] != class
        {
            return Err(StoreError::Rejected);
        }
        unclassified += u64::from(class == "unclassified");
        retired += u64::from(class == "terminally_retired_no_replay");
        pending += u64::from(class == "pending_accepted_undelivered");
        stored_rows.push((
            table.into(),
            identity.into(),
            revision,
            projection.into(),
            class.into(),
            serde_json::to_vec(row).map_err(|_| StoreError::Rejected)?,
        ));
    }
    if unclassified == 0 || retired == 0 || pending == 0 {
        return Err(StoreError::Rejected);
    }
    let mut stored_artifacts = Vec::with_capacity(HELD_ARTIFACT_NAMES.len());
    for name in HELD_ARTIFACT_NAMES {
        let bytes = raw(name)?;
        stored_artifacts.push((name.to_owned(), digest(bytes)?, bytes.to_vec()));
    }
    Ok((
        stored_artifacts,
        stored_rows,
        digest(raw("source/capture/source-manifest.json")?)?,
        digest(capsule_raw)?,
        digest(inventory_raw)?,
        unclassified,
    ))
}

pub(crate) fn check_held_coherence(connection: &rusqlite::Connection) -> Result<(), StoreError> {
    let marker_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM legacy_held_state", [], |r| r.get(0))
        .map_err(|_| StoreError::Unavailable)?;
    let guard_count: i64 = connection.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name GLOB 'held_quarantine_*'", [], |r| r.get(0)).map_err(|_| StoreError::Unavailable)?;
    if marker_count != 1 || guard_count != 93 {
        return Err(StoreError::Unavailable);
    }
    let (mode, handoff, rows, unknown): (String, Option<String>, i64, i64) = connection
        .query_row(
            "SELECT mode,handoff_id,row_count,unclassified_count FROM legacy_held_state WHERE singleton=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|_| StoreError::Unavailable)?;
    let actual_rows: i64 = connection
        .query_row("SELECT COUNT(*) FROM legacy_held_rows", [], |r| r.get(0))
        .map_err(|_| StoreError::Unavailable)?;
    let artifact_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM legacy_held_artifacts", [], |r| {
            r.get(0)
        })
        .map_err(|_| StoreError::Unavailable)?;
    let lineage_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM legacy_held_lineage", [], |r| r.get(0))
        .map_err(|_| StoreError::Unavailable)?;
    if mode == "ordinary" {
        if rows != 0
            || unknown != 0
            || actual_rows != 0
            || artifact_count != 0
            || lineage_count != 0
        {
            return Err(StoreError::Unavailable);
        }
        return Ok(());
    }
    if mode != "held_quarantine" || actual_rows != rows || artifact_count != 16 {
        return Err(StoreError::Unavailable);
    }
    let handoff = handoff.ok_or(StoreError::Unavailable)?;
    let (current_store, custody_revision): (String, i64) = connection
        .query_row("SELECT s.store_id,c.revision FROM store_identity s JOIN admin_custody_revision c ON c.singleton=s.singleton WHERE s.singleton=1", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(|_| StoreError::Unavailable)?;
    let immediate: Option<(String, String, String, i64)> = connection
        .query_row("SELECT source_store_id,capsule_sha256,handoff_id,source_revision FROM admin_store_lineage WHERE singleton=1", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .optional()
        .map_err(|_| StoreError::Unavailable)?;
    let mut lineage_stmt = connection
        .prepare("SELECT ordinal,source_store_id,destination_store_id,source_revision,capsule_sha256,handoff_id FROM legacy_held_lineage ORDER BY ordinal")
        .map_err(|_| StoreError::Unavailable)?;
    let lineage_rows = lineage_stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
            ))
        })
        .map_err(|_| StoreError::Unavailable)?;
    let mut prior_destination: Option<String> = None;
    let mut prior_revision = -1_i64;
    let mut last: Option<(String, String, String, i64)> = None;
    let mut observed_lineage = 0_i64;
    for (expected, row) in lineage_rows.enumerate() {
        let (ordinal, source, destination, revision, capsule, lineage_handoff) =
            row.map_err(|_| StoreError::Unavailable)?;
        if ordinal != i64::try_from(expected).map_err(|_| StoreError::Unavailable)?
            || source == destination
            || prior_destination
                .as_ref()
                .is_some_and(|prior| prior != &source)
            || revision <= prior_revision
            || revision >= custody_revision
            || !hex_digest(&capsule)
            || lineage_handoff != handoff
        {
            return Err(StoreError::Unavailable);
        }
        prior_destination = Some(destination);
        prior_revision = revision;
        last = Some((source, capsule, lineage_handoff, revision));
        observed_lineage += 1;
    }
    if observed_lineage != lineage_count
        || prior_destination
            .as_ref()
            .is_some_and(|id| id != &current_store)
        || immediate != last
    {
        return Err(StoreError::Unavailable);
    }
    let actual_unknown: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM legacy_held_rows WHERE classification='unclassified'",
            [],
            |r| r.get(0),
        )
        .map_err(|_| StoreError::Unavailable)?;
    if actual_unknown != unknown {
        return Err(StoreError::Unavailable);
    }
    // All normal state remains empty while its source effects are unresolved.
    for table in [
        "artifacts",
        "attempts",
        "commands",
        "effects",
        "grant_contractions",
        "grants",
        "inputs",
        "interruption_requests",
        "journal",
        "native_delivery_segments",
        "native_ingress",
        "native_ingress_conflicts",
        "native_operations",
        "native_process_exits",
        "native_sessions",
        "native_task_operations",
        "native_turns",
        "native_usage",
        "native_verifier_results",
        "observation_conflicts",
        "observations",
        "semantic_attempts",
        "semantic_captures",
        "semantic_material_rejections",
        "semantic_results",
        "service_closes",
        "subjects",
        "task_results",
        "transition_evidence",
        "transitions",
        "verifier_results",
    ] {
        let sql = format!("SELECT COUNT(*) FROM {table}");
        let count: i64 = connection
            .query_row(&sql, [], |r| r.get(0))
            .map_err(|_| StoreError::Unavailable)?;
        if count != 0 {
            return Err(StoreError::Unavailable);
        }
    }
    let mut statement = connection
        .prepare(
            "SELECT name,sha256,byte_length,raw_bytes FROM legacy_held_artifacts ORDER BY name",
        )
        .map_err(|_| StoreError::Unavailable)?;
    let records = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, Vec<u8>>(3)?,
            ))
        })
        .map_err(|_| StoreError::Unavailable)?;
    let mut seen = std::collections::BTreeSet::new();
    for record in records {
        let (name, sha, len, bytes) = record.map_err(|_| StoreError::Unavailable)?;
        if !HELD_ARTIFACT_NAMES.contains(&name.as_str())
            || !seen.insert(name)
            || len != bytes.len() as i64
            || digest(&bytes)? != sha
        {
            return Err(StoreError::Unavailable);
        }
    }
    if seen.len() != 16 {
        return Err(StoreError::Unavailable);
    }
    let mut rows_stmt = connection.prepare("SELECT table_name,source_identity,source_revision,source_projection_sha256,classification,row_json FROM legacy_held_rows ORDER BY ordinal")
        .map_err(|_| StoreError::Unavailable)?;
    let row_records = rows_stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, Vec<u8>>(5)?,
            ))
        })
        .map_err(|_| StoreError::Unavailable)?;
    for row in row_records {
        let (table, id, rev, projection, class, raw) = row.map_err(|_| StoreError::Unavailable)?;
        let value: serde_json::Value =
            serde_json::from_slice(&raw).map_err(|_| StoreError::Unavailable)?;
        if value["table"] != table
            || value["identity"] != id
            || value["revision"].as_str() != rev.as_deref()
            || value["source_projection_sha256"] != projection
            || value["classification"] != class
        {
            return Err(StoreError::Unavailable);
        }
    }
    let mut artifact_stmt = connection
        .prepare("SELECT name,raw_bytes FROM legacy_held_artifacts ORDER BY name")
        .map_err(|_| StoreError::Unavailable)?;
    let artifact_inputs = artifact_stmt
        .query_map([], |r| {
            Ok(HeldArtifactInput {
                name: r.get(0)?,
                raw_bytes: r.get(1)?,
            })
        })
        .map_err(|_| StoreError::Unavailable)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StoreError::Unavailable)?;
    let (_, expected_rows, capture_sha, capsule_sha, inventory_sha, expected_unknown) =
        validate_held_evidence(&HeldEvidence {
            artifacts: artifact_inputs,
        })
        .map_err(|_| StoreError::Unavailable)?;
    let (saved_capture, saved_capsule, saved_inventory): (String,String,String) = connection.query_row(
        "SELECT source_capture_sha256,capsule_sha256,inventory_sha256 FROM legacy_held_state WHERE singleton=1",
        [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
    ).map_err(|_| StoreError::Unavailable)?;
    if capture_sha != saved_capture
        || capsule_sha != saved_capsule
        || inventory_sha != saved_inventory
        || expected_rows.len() as i64 != rows
        || expected_unknown as i64 != unknown
    {
        return Err(StoreError::Unavailable);
    }
    let mut stored_stmt = connection.prepare("SELECT table_name,source_identity,source_revision,source_projection_sha256,classification,row_json FROM legacy_held_rows ORDER BY ordinal")
        .map_err(|_| StoreError::Unavailable)?;
    let stored_rows = stored_stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })
        .map_err(|_| StoreError::Unavailable)?
        .collect::<Result<Vec<HeldRow>, _>>()
        .map_err(|_| StoreError::Unavailable)?;
    if stored_rows != expected_rows {
        return Err(StoreError::Unavailable);
    }
    Ok(())
}

fn held_projection_from(connection: &rusqlite::Connection) -> Result<HeldProjection, StoreError> {
    let (mode, handoff, capture, capsule, inventory, rows, unknown): HeldMarkerRow = connection.query_row(
            "SELECT mode,handoff_id,source_capture_sha256,capsule_sha256,inventory_sha256,row_count,unclassified_count FROM legacy_held_state WHERE singleton=1",
            [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?)),
        ).map_err(|_| StoreError::Unavailable)?;
    if mode != "held_quarantine" {
        return Err(StoreError::Rejected);
    }
    let (store_id, ownership_status, revision): (String, String, i64) = connection.query_row(
        "SELECT s.store_id,o.status,c.revision FROM store_identity s JOIN admin_ownership o ON o.singleton=s.singleton JOIN admin_custody_revision c ON c.singleton=s.singleton WHERE s.singleton=1",
        [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
    ).map_err(|_| StoreError::Unavailable)?;
    let mut stmt = connection
        .prepare("SELECT name,sha256 FROM legacy_held_artifacts ORDER BY name")
        .map_err(|_| StoreError::Unavailable)?;
    let artifact_digests = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(|_| StoreError::Unavailable)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StoreError::Unavailable)?;
    Ok(HeldProjection {
        store_id,
        ownership_status,
        handoff_id: handoff.ok_or(StoreError::Unavailable)?,
        source_capture_sha256: capture.ok_or(StoreError::Unavailable)?,
        capsule_sha256: capsule.ok_or(StoreError::Unavailable)?,
        inventory_sha256: inventory.ok_or(StoreError::Unavailable)?,
        row_count: u64::try_from(rows).map_err(|_| StoreError::Unavailable)?,
        unclassified_count: u64::try_from(unknown).map_err(|_| StoreError::Unavailable)?,
        custody_revision: u64::try_from(revision).map_err(|_| StoreError::Unavailable)?,
        artifact_digests,
    })
}

fn held_receipt_digest(receipt: &HeldCapsuleReceipt) -> Result<String, StoreError> {
    let mut unsigned = receipt.clone();
    unsigned.receipt_sha256.clear();
    digest(&serde_json::to_vec(&unsigned).map_err(|_| StoreError::Unavailable)?)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
enum HeldReservationOperation {
    Bootstrap {
        evidence_sha256: String,
    },
    Adopt {
        receipt_sha256: String,
        source_store_id: String,
        source_revision: u64,
        source_custody_phase: String,
        source_database_sha256: String,
        capsule_root: PathBuf,
    },
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct HeldDestinationReservation {
    format: String,
    destination: PathBuf,
    stage: PathBuf,
    handoff_id: String,
    source_capture_sha256: String,
    capsule_sha256: String,
    inventory_sha256: String,
    trusted_issuer: String,
    operation: HeldReservationOperation,
}

fn held_evidence_digest(evidence: &HeldEvidence) -> Result<String, StoreError> {
    let mut members = evidence
        .artifacts
        .iter()
        .map(|a| Ok((a.name.as_str(), a.raw_bytes.len(), digest(&a.raw_bytes)?)))
        .collect::<Result<Vec<_>, StoreError>>()?;
    members.sort();
    digest(&serde_json::to_vec(&members).map_err(|_| StoreError::Unavailable)?)
}

fn held_destination_reservation_path(destination: &Path) -> Result<PathBuf, StoreError> {
    let parent = destination.parent().ok_or(StoreError::Rejected)?;
    let name = destination.file_name().ok_or(StoreError::Rejected)?;
    Ok(parent.join(format!(
        ".{}.held-destination-reservation.json",
        name.to_string_lossy()
    )))
}

fn create_held_destination_reservation(
    destination: &Path,
    witness: &HeldSourceWitness,
    inventory_sha256: &str,
    trusted_issuer: &str,
    operation: HeldReservationOperation,
) -> Result<(File, PathBuf), StoreError> {
    check_held_destination_path(destination)?;
    let stable = held_destination_reservation_path(destination)?;
    let parent = destination.parent().ok_or(StoreError::Rejected)?;
    if !stable.exists() {
        if destination.exists() {
            return Err(StoreError::ClaimConflict);
        }
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|_| StoreError::Unavailable)?;
        let suffix = random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let name = destination.file_name().ok_or(StoreError::Rejected)?;
        let stage = parent.join(format!(".{}.held-stage-{suffix}", name.to_string_lossy()));
        let candidate = HeldDestinationReservation {
            format: "legacy-v28-held-destination-reservation-v1".into(),
            destination: destination.to_path_buf(),
            stage,
            handoff_id: witness.handoff_id.clone(),
            source_capture_sha256: witness.source_capture_sha256.clone(),
            capsule_sha256: witness.capsule_sha256.clone(),
            inventory_sha256: inventory_sha256.into(),
            trusted_issuer: trusted_issuer.into(),
            operation: operation.clone(),
        };
        let temporary = parent.join(format!(
            ".{}.held-reservation-{suffix}",
            name.to_string_lossy()
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| StoreError::ClaimConflict)?;
        file.write_all(&serde_json::to_vec(&candidate).map_err(|_| StoreError::Unavailable)?)
            .map_err(|_| StoreError::Unavailable)?;
        file.sync_all().map_err(|_| StoreError::Unavailable)?;
        if publish_held_stage(&temporary, &stable).is_err() {
            // Another exact reservation may have won. No stage exists for our
            // unpublished candidate, so its temporary file can be removed.
            fs::remove_file(&temporary).map_err(|_| StoreError::Unavailable)?;
            if !stable.exists() {
                return Err(StoreError::Unavailable);
            }
        }
    }
    if !fs::symlink_metadata(&stable)
        .map_err(|_| StoreError::Unavailable)?
        .file_type()
        .is_file()
    {
        return Err(StoreError::ClaimConflict);
    }
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&stable)
        .map_err(|_| StoreError::Unavailable)?;
    lock.try_lock().map_err(|_| StoreError::ClaimConflict)?;
    let saved: HeldDestinationReservation =
        serde_json::from_slice(&fs::read(&stable).map_err(|_| StoreError::Unavailable)?)
            .map_err(|_| StoreError::ClaimConflict)?;
    let name = destination.file_name().ok_or(StoreError::Rejected)?;
    let stage_name = saved
        .stage
        .file_name()
        .ok_or(StoreError::ClaimConflict)?
        .to_string_lossy();
    let stage_prefix = format!(".{}.held-stage-", name.to_string_lossy());
    let suffix = stage_name
        .strip_prefix(&stage_prefix)
        .ok_or(StoreError::ClaimConflict)?;
    if saved.format != "legacy-v28-held-destination-reservation-v1"
        || saved.destination != destination
        || saved.stage.parent() != Some(parent)
        || suffix.len() != 32
        || !suffix.bytes().all(|b| b.is_ascii_hexdigit())
        || saved.handoff_id != witness.handoff_id
        || saved.source_capture_sha256 != witness.source_capture_sha256
        || saved.capsule_sha256 != witness.capsule_sha256
        || saved.inventory_sha256 != inventory_sha256
        || saved.trusted_issuer != trusted_issuer
        || saved.operation != operation
    {
        return Err(StoreError::ClaimConflict);
    }
    Ok((lock, saved.stage))
}

fn ensure_held_destination_stage(stage: &Path) -> Result<(), StoreError> {
    let intent = crate::sqlite::adoption_intent_path(stage)?;
    let parent = stage.parent().ok_or(StoreError::Rejected)?;
    let marker = b"legacy-v28-held-destination-stage-v1";
    if !intent.exists() {
        if stage.exists() {
            return Err(StoreError::ClaimConflict);
        }
        let mut random = [0u8; 16];
        getrandom::fill(&mut random).map_err(|_| StoreError::Unavailable)?;
        let suffix = random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let temporary = parent.join(format!(
            "{}.preparing-{suffix}",
            intent
                .file_name()
                .ok_or(StoreError::Rejected)?
                .to_string_lossy()
        ));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| StoreError::ClaimConflict)?;
        file.write_all(marker)
            .map_err(|_| StoreError::Unavailable)?;
        file.sync_all().map_err(|_| StoreError::Unavailable)?;
        publish_held_stage(&temporary, &intent)?;
    }
    if !fs::symlink_metadata(&intent)
        .map_err(|_| StoreError::Unavailable)?
        .file_type()
        .is_file()
        || fs::read(&intent).map_err(|_| StoreError::Unavailable)? != marker
    {
        return Err(StoreError::ClaimConflict);
    }
    if !stage.exists() {
        fs::create_dir(stage).map_err(|_| StoreError::ClaimConflict)?;
        sync_dir(parent)?;
    } else if !fs::symlink_metadata(stage)
        .map_err(|_| StoreError::Unavailable)?
        .file_type()
        .is_dir()
        || fs::canonicalize(stage).map_err(|_| StoreError::Unavailable)? != stage
    {
        return Err(StoreError::ClaimConflict);
    }
    Ok(())
}

fn ensure_held_adoption_copy(stage: &Path, capsule: &HeldCapsuleReceipt) -> Result<(), StoreError> {
    ensure_held_destination_stage(stage)?;
    let destination = stage.join("lifecycle.sqlite");
    if destination.exists() {
        if !fs::symlink_metadata(&destination)
            .map_err(|_| StoreError::Unavailable)?
            .file_type()
            .is_file()
        {
            return Err(StoreError::ClaimConflict);
        }
        if fs::symlink_metadata(stage.join(".lifecycle.sqlite.copying")).is_ok() {
            return Err(StoreError::ClaimConflict);
        }
        return Ok(());
    }
    let temporary = stage.join(".lifecycle.sqlite.copying");
    if let Ok(meta) = fs::symlink_metadata(&temporary) {
        if !meta.file_type().is_file() {
            return Err(StoreError::ClaimConflict);
        }
        // Only an incomplete copy inside this locked, unpublished stage is
        // replaceable. The reservation, stage and external intent survive.
        fs::remove_file(&temporary).map_err(|_| StoreError::Unavailable)?;
    }
    durable_copy(&capsule.capsule_root.join("lifecycle.sqlite"), &temporary)?;
    if digest(&fs::read(&temporary).map_err(|_| StoreError::Unavailable)?)?
        != capsule.database_sha256
    {
        return Err(StoreError::ClaimConflict);
    }
    publish_held_stage(&temporary, &destination)
}

fn check_held_destination_path(destination: &Path) -> Result<(), StoreError> {
    if !destination.is_absolute() {
        return Err(StoreError::Rejected);
    }
    let parent = destination.parent().ok_or(StoreError::Rejected)?;
    if !fs::symlink_metadata(parent)
        .map_err(|_| StoreError::Unavailable)?
        .file_type()
        .is_dir()
        || fs::canonicalize(parent).map_err(|_| StoreError::Unavailable)? != parent
    {
        return Err(StoreError::Rejected);
    }
    if let Ok(meta) = fs::symlink_metadata(destination)
        && (!meta.file_type().is_dir()
            || fs::canonicalize(destination).map_err(|_| StoreError::Unavailable)? != destination)
    {
        return Err(StoreError::Rejected);
    }
    Ok(())
}

fn inspect_existing_held_root(root: &Path) -> Result<HeldProjection, StoreError> {
    let db = root.join("lifecycle.sqlite");
    if !fs::symlink_metadata(&db)
        .map_err(|_| StoreError::ClaimConflict)?
        .file_type()
        .is_file()
    {
        return Err(StoreError::Rejected);
    }
    let connection = rusqlite::Connection::open_with_flags(
        &db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(|_| StoreError::Unavailable)?;
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|_| StoreError::Unavailable)?;
    if version != 6 {
        return Err(StoreError::ClaimConflict);
    }
    crate::recovery::check_coherence(&connection)?;
    held_projection_from(&connection)
}

#[cfg(target_os = "linux")]
fn publish_held_stage(stage: &Path, destination: &Path) -> Result<(), StoreError> {
    use std::os::unix::ffi::OsStrExt;
    unsafe extern "C" {
        fn renameat2(
            olddirfd: i32,
            oldpath: *const i8,
            newdirfd: i32,
            newpath: *const i8,
            flags: u32,
        ) -> i32;
    }
    let old =
        std::ffi::CString::new(stage.as_os_str().as_bytes()).map_err(|_| StoreError::Rejected)?;
    let new = std::ffi::CString::new(destination.as_os_str().as_bytes())
        .map_err(|_| StoreError::Rejected)?;
    // RENAME_NOREPLACE prevents an overlapping actor's empty root from being replaced.
    let changed = unsafe { renameat2(-100, old.as_ptr(), -100, new.as_ptr(), 1) };
    if changed != 0 {
        return Err(StoreError::ClaimConflict);
    }
    sync_dir(destination.parent().ok_or(StoreError::Rejected)?)
}

#[cfg(not(target_os = "linux"))]
fn publish_held_stage(_stage: &Path, _destination: &Path) -> Result<(), StoreError> {
    Err(StoreError::Rejected)
}

impl SqliteLifecycleStore {
    /// Validate A-v2-shaped bytes without creating a destination or authority.
    pub fn inspect_held_evidence(evidence: &HeldEvidence) -> Result<(u64, u64), StoreError> {
        let (_, rows, _, _, _, unknown) = validate_held_evidence(evidence)?;
        Ok((rows.len() as u64, unknown))
    }

    /// A read-only held projection, separate from normal recovery work.
    pub fn held_projection(&self) -> Result<HeldProjection, StoreError> {
        self.verify_identity()?;
        crate::recovery::check_coherence(&self.connection)?;
        held_projection_from(&self.connection)
    }

    /// No production caller can mint the required witness in B1.
    pub fn bootstrap_held(
        witness: &HeldSourceWitness,
        evidence: &HeldEvidence,
        destination: &Path,
        trusted_issuer: String,
    ) -> Result<Self, StoreError> {
        check_held_destination_path(destination)?;
        if trusted_issuer.is_empty() || witness.handoff_id.is_empty() {
            return Err(StoreError::Rejected);
        }
        let (artifacts, rows, capture_sha, capsule_sha, inventory_sha, unknown) =
            validate_held_evidence(evidence)?;
        if capture_sha != witness.source_capture_sha256 || capsule_sha != witness.capsule_sha256 {
            return Err(StoreError::Rejected);
        }
        let evidence_sha = held_evidence_digest(evidence)?;
        let (_reservation_lock, stage) = create_held_destination_reservation(
            destination,
            witness,
            &inventory_sha,
            &trusted_issuer,
            HeldReservationOperation::Bootstrap {
                evidence_sha256: evidence_sha,
            },
        )?;
        #[cfg(test)]
        if std::env::var("LIFECYCLE_HELD_TEST_CUT").as_deref() == Ok("after_reservation") {
            std::process::exit(84);
        }
        if destination.exists() {
            let projection = inspect_existing_held_root(destination)?;
            if projection.handoff_id == witness.handoff_id
                && projection.source_capture_sha256 == capture_sha
                && projection.capsule_sha256 == capsule_sha
                && projection.inventory_sha256 == inventory_sha
                && projection.ownership_status == "inactive"
            {
                let store = Self::open(destination, trusted_issuer)?;
                if store.held_projection()? != projection {
                    return Err(StoreError::ClaimConflict);
                }
                let stage_intent = crate::sqlite::adoption_intent_path(&stage)?;
                if stage_intent.exists() && !stage.exists() {
                    fs::remove_file(stage_intent).map_err(|_| StoreError::Unavailable)?;
                    sync_dir(destination.parent().ok_or(StoreError::Rejected)?)?;
                }
                return Ok(store);
            }
            return Err(StoreError::ClaimConflict);
        }
        ensure_held_destination_stage(&stage)?;
        {
            let mut store = Self::open_adoption_pending(&stage, trusted_issuer.clone())?;
            #[cfg(test)]
            if std::env::var("LIFECYCLE_HELD_TEST_CUT").as_deref() == Ok("after_stage_open") {
                std::process::exit(83);
            }
            if held_quarantine(&store.connection)? {
                let projection = store.held_projection()?;
                let mut expected_digests = artifacts
                    .iter()
                    .map(|(name, sha, _)| (name.clone(), sha.clone()))
                    .collect::<Vec<_>>();
                expected_digests.sort();
                if projection.handoff_id != witness.handoff_id
                    || projection.source_capture_sha256 != capture_sha
                    || projection.capsule_sha256 != capsule_sha
                    || projection.inventory_sha256 != inventory_sha
                    || projection.row_count != rows.len() as u64
                    || projection.unclassified_count != unknown
                    || projection.ownership_status != "inactive"
                    || projection.artifact_digests != expected_digests
                {
                    return Err(StoreError::ClaimConflict);
                }
            } else {
                let tx = store
                    .connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|_| StoreError::Unavailable)?;
                for (name, sha, bytes) in &artifacts {
                    tx.execute("INSERT INTO legacy_held_artifacts(name,sha256,byte_length,raw_bytes) VALUES (?1,?2,?3,?4)",
                    params![name,sha,i64::try_from(bytes.len()).map_err(|_| StoreError::Rejected)?,bytes])
                    .map_err(|_| StoreError::Unavailable)?;
                }
                for (ordinal, (table, id, revision, projection, class, raw)) in
                    rows.iter().enumerate()
                {
                    tx.execute("INSERT INTO legacy_held_rows(ordinal,table_name,source_identity,source_revision,source_projection_sha256,classification,row_json) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                    params![i64::try_from(ordinal).map_err(|_| StoreError::Rejected)?,table,id,revision,projection,class,raw])
                    .map_err(|_| StoreError::Unavailable)?;
                }
                tx.execute("UPDATE admin_ownership SET status='inactive',revision=revision+1,handoff_id=?1,capsule_sha256=?2,peer_store_id=NULL,selection_revision=NULL WHERE singleton=1 AND status='active'",
                params![&witness.handoff_id,&capsule_sha]).map_err(|_| StoreError::Unavailable)?;
                tx.execute("UPDATE legacy_held_state SET mode='held_quarantine',handoff_id=?1,source_capture_sha256=?2,capsule_sha256=?3,inventory_sha256=?4,row_count=?5,unclassified_count=?6 WHERE singleton=1 AND mode='ordinary'",
                params![&witness.handoff_id,&capture_sha,&capsule_sha,&inventory_sha,i64::try_from(rows.len()).map_err(|_| StoreError::Rejected)?,i64::try_from(unknown).map_err(|_| StoreError::Rejected)?])
                .map_err(|_| StoreError::Unavailable)?;
                tx.commit().map_err(|_| StoreError::Unavailable)?;
            }
            let projection = store.held_projection()?;
            if projection.row_count != rows.len() as u64
                || projection.unclassified_count != unknown
                || projection.ownership_status != "inactive"
            {
                return Err(StoreError::Unavailable);
            }
            store
                .connection
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .map_err(|_| StoreError::Unavailable)?;
        }
        sync_dir(&stage)?;
        #[cfg(test)]
        if std::env::var("LIFECYCLE_HELD_TEST_CUT").as_deref() == Ok("before_publish") {
            std::process::exit(81);
        }
        publish_held_stage(&stage, destination)?;
        #[cfg(test)]
        if std::env::var("LIFECYCLE_HELD_TEST_CUT").as_deref() == Ok("after_publish") {
            std::process::exit(82);
        }
        fs::remove_file(crate::sqlite::adoption_intent_path(&stage)?)
            .map_err(|_| StoreError::Unavailable)?;
        sync_dir(destination.parent().ok_or(StoreError::Rejected)?)?;
        let store = Self::open(destination, trusted_issuer)?;
        store.held_projection()?;
        Ok(store)
    }

    /// Distinct held format. Ordinary export_current still refuses this root.
    pub fn export_held(&self, destination: &Path) -> Result<HeldCapsuleReceipt, StoreError> {
        let projection = self.held_projection()?;
        if projection.ownership_status != "inactive" && projection.ownership_status != "active" {
            return Err(StoreError::Rejected);
        }
        if destination.exists() {
            let prior = Self::read_held_capsule(destination)?;
            if prior.source_store_id == projection.store_id
                && prior.source_revision == projection.custody_revision
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
        let db_bytes =
            fs::read(destination.join("lifecycle.sqlite")).map_err(|_| StoreError::Unavailable)?;
        let mut receipt = HeldCapsuleReceipt {
            format: "legacy-v28-held-custody-v1".into(),
            capsule_root: destination.to_path_buf(),
            source_store_id: projection.store_id,
            source_revision: projection.custody_revision,
            source_custody_phase: if projection.ownership_status == "inactive" {
                "inactive".into()
            } else {
                "recovery_only".into()
            },
            database_sha256: digest(&db_bytes)?,
            handoff_id: projection.handoff_id,
            source_capture_sha256: projection.source_capture_sha256,
            origin_capsule_sha256: projection.capsule_sha256,
            inventory_sha256: projection.inventory_sha256,
            row_count: projection.row_count,
            unclassified_count: projection.unclassified_count,
            artifact_digests: projection.artifact_digests,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = held_receipt_digest(&receipt)?;
        let bytes = serde_json::to_vec(&receipt).map_err(|_| StoreError::Unavailable)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(destination.join("held-manifest.json"))
            .map_err(|_| StoreError::Unavailable)?;
        file.write_all(&bytes)
            .map_err(|_| StoreError::Unavailable)?;
        file.sync_all().map_err(|_| StoreError::Unavailable)?;
        sync_dir(destination)?;
        let readback = Self::read_held_capsule(destination)?;
        if readback != receipt {
            return Err(StoreError::Unavailable);
        }
        Ok(receipt)
    }

    pub fn read_held_capsule(root: &Path) -> Result<HeldCapsuleReceipt, StoreError> {
        if !fs::symlink_metadata(root)
            .map_err(|_| StoreError::Unavailable)?
            .file_type()
            .is_dir()
        {
            return Err(StoreError::Rejected);
        }
        let receipt: HeldCapsuleReceipt = serde_json::from_slice(
            &fs::read(root.join("held-manifest.json")).map_err(|_| StoreError::Unavailable)?,
        )
        .map_err(|_| StoreError::Rejected)?;
        if receipt.format != "legacy-v28-held-custody-v1"
            || receipt.capsule_root != root
            || receipt.receipt_sha256 != held_receipt_digest(&receipt)?
        {
            return Err(StoreError::Rejected);
        }
        let db = root.join("lifecycle.sqlite");
        if !fs::symlink_metadata(&db)
            .map_err(|_| StoreError::Unavailable)?
            .file_type()
            .is_file()
            || digest(&fs::read(&db).map_err(|_| StoreError::Unavailable)?)?
                != receipt.database_sha256
        {
            return Err(StoreError::Rejected);
        }
        let connection = rusqlite::Connection::open_with_flags(
            &db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|_| StoreError::Unavailable)?;
        crate::recovery::check_coherence(&connection)?;
        let projection = held_projection_from(&connection)?;
        if projection.store_id != receipt.source_store_id
            || projection.custody_revision != receipt.source_revision
            || (if projection.ownership_status == "inactive" {
                "inactive"
            } else if projection.ownership_status == "active" {
                "recovery_only"
            } else {
                "invalid"
            }) != receipt.source_custody_phase
            || projection.handoff_id != receipt.handoff_id
            || projection.source_capture_sha256 != receipt.source_capture_sha256
            || projection.capsule_sha256 != receipt.origin_capsule_sha256
            || projection.inventory_sha256 != receipt.inventory_sha256
            || projection.row_count != receipt.row_count
            || projection.unclassified_count != receipt.unclassified_count
            || projection.artifact_digests != receipt.artifact_digests
        {
            return Err(StoreError::Rejected);
        }
        Ok(receipt)
    }

    /// A held snapshot can only be adopted under a witness; the result remains inactive.
    pub fn adopt_held_inactive(
        witness: &HeldSourceWitness,
        capsule: &HeldCapsuleReceipt,
        destination: &Path,
        trusted_issuer: String,
    ) -> Result<Self, StoreError> {
        check_held_destination_path(destination)?;
        if trusted_issuer.is_empty() {
            return Err(StoreError::Rejected);
        }
        let checked = Self::read_held_capsule(&capsule.capsule_root)?;
        if &checked != capsule
            || witness.handoff_id != capsule.handoff_id
            || witness.source_capture_sha256 != capsule.source_capture_sha256
            || witness.capsule_sha256 != capsule.origin_capsule_sha256
        {
            return Err(StoreError::ClaimConflict);
        }
        let (_reservation_lock, stage) = create_held_destination_reservation(
            destination,
            witness,
            &capsule.inventory_sha256,
            &trusted_issuer,
            HeldReservationOperation::Adopt {
                receipt_sha256: capsule.receipt_sha256.clone(),
                source_store_id: capsule.source_store_id.clone(),
                source_revision: capsule.source_revision,
                source_custody_phase: capsule.source_custody_phase.clone(),
                source_database_sha256: capsule.database_sha256.clone(),
                capsule_root: capsule.capsule_root.clone(),
            },
        )?;
        #[cfg(test)]
        if std::env::var("LIFECYCLE_HELD_TEST_CUT").as_deref() == Ok("adopt_after_reservation") {
            std::process::exit(94);
        }
        if destination.exists() {
            let projection = inspect_existing_held_root(destination)?;
            let store = Self::open(destination, trusted_issuer)?;
            if store.held_projection()? != projection {
                return Err(StoreError::ClaimConflict);
            }
            let lineage: Option<(String,String,i64)> = store.connection.query_row(
                "SELECT source_store_id,capsule_sha256,source_revision FROM admin_store_lineage WHERE singleton=1", [],
                |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
            ).optional().map_err(|_| StoreError::Unavailable)?;
            if projection.ownership_status == "inactive"
                && projection.handoff_id == witness.handoff_id
                && projection.source_capture_sha256 == witness.source_capture_sha256
                && lineage
                    == Some((
                        capsule.source_store_id.clone(),
                        capsule.receipt_sha256.clone(),
                        to_i64(capsule.source_revision)?,
                    ))
            {
                if stage.exists() {
                    return Err(StoreError::ClaimConflict);
                }
                let intent = crate::sqlite::adoption_intent_path(&stage)?;
                if intent.exists() {
                    fs::remove_file(intent).map_err(|_| StoreError::Unavailable)?;
                    sync_dir(destination.parent().ok_or(StoreError::Rejected)?)?;
                }
                return Ok(store);
            }
            return Err(StoreError::ClaimConflict);
        }
        ensure_held_adoption_copy(&stage, capsule)?;
        #[cfg(test)]
        if std::env::var("LIFECYCLE_HELD_TEST_CUT").as_deref() == Ok("adopt_after_copy") {
            std::process::exit(93);
        }
        {
            let mut store = Self::open_adoption_pending(&stage, trusted_issuer.clone())?;
            let prior = store.held_projection()?;
            #[cfg(test)]
            if std::env::var("LIFECYCLE_HELD_TEST_CUT").as_deref() == Ok("adopt_after_stage_open") {
                std::process::exit(92);
            }
            if prior.handoff_id != witness.handoff_id
                || prior.source_capture_sha256 != witness.source_capture_sha256
                || prior.capsule_sha256 != witness.capsule_sha256
                || prior.inventory_sha256 != capsule.inventory_sha256
                || prior.row_count != capsule.row_count
                || prior.unclassified_count != capsule.unclassified_count
                || prior.artifact_digests != capsule.artifact_digests
            {
                return Err(StoreError::ClaimConflict);
            }
            if prior.store_id == capsule.source_store_id {
                if prior.custody_revision != capsule.source_revision
                    || (if prior.ownership_status == "inactive" {
                        "inactive"
                    } else if prior.ownership_status == "active" {
                        "recovery_only"
                    } else {
                        "invalid"
                    }) != capsule.source_custody_phase
                {
                    return Err(StoreError::ClaimConflict);
                }
                let prior_lineage_count: i64 = store
                    .connection
                    .query_row("SELECT COUNT(*) FROM legacy_held_lineage", [], |r| r.get(0))
                    .map_err(|_| StoreError::Unavailable)?;
                let mut random = [0u8; 16];
                getrandom::fill(&mut random).map_err(|_| StoreError::Unavailable)?;
                let new_id = format!(
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
                    [&new_id],
                )
                .map_err(|_| StoreError::Unavailable)?;
                tx.execute("UPDATE admin_ownership SET status='inactive',selection_revision=NULL,revision=revision+1 WHERE singleton=1 AND status IN ('inactive','active')",[])
                    .map_err(|_| StoreError::Unavailable)?;
                tx.execute("INSERT INTO admin_store_lineage(singleton,source_store_id,capsule_sha256,handoff_id,source_revision) VALUES (1,?1,?2,?3,?4) ON CONFLICT(singleton) DO UPDATE SET source_store_id=excluded.source_store_id,capsule_sha256=excluded.capsule_sha256,handoff_id=excluded.handoff_id,source_revision=excluded.source_revision",
                    params![&capsule.source_store_id,&capsule.receipt_sha256,&witness.handoff_id,to_i64(capsule.source_revision)?])
                    .map_err(|_| StoreError::Unavailable)?;
                tx.execute("INSERT INTO legacy_held_lineage(ordinal,source_store_id,destination_store_id,source_revision,capsule_sha256,handoff_id) VALUES (?1,?2,?3,?4,?5,?6)",
                    params![prior_lineage_count,&capsule.source_store_id,&new_id,to_i64(capsule.source_revision)?,&capsule.receipt_sha256,&witness.handoff_id])
                    .map_err(|_| StoreError::Unavailable)?;
                tx.commit().map_err(|_| StoreError::Unavailable)?;
                #[cfg(test)]
                if std::env::var("LIFECYCLE_HELD_TEST_CUT").as_deref()
                    == Ok("adopt_after_lineage_commit")
                {
                    std::process::exit(91);
                }
            }
            let after = store.held_projection()?;
            let latest: (String, String, i64, String, String, String) = store
                .connection
                .query_row(
                    "SELECT source_store_id,destination_store_id,source_revision,capsule_sha256,handoff_id,(SELECT capsule_sha256 FROM admin_store_lineage WHERE singleton=1) FROM legacy_held_lineage ORDER BY ordinal DESC LIMIT 1",
                    [],
                    |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)),
                )
                .map_err(|_| StoreError::Unavailable)?;
            let singleton: (String, String, String, i64) = store
                .connection
                .query_row(
                    "SELECT source_store_id,capsule_sha256,handoff_id,source_revision FROM admin_store_lineage WHERE singleton=1",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
                )
                .map_err(|_| StoreError::Unavailable)?;
            if after.store_id == capsule.source_store_id
                || after.custody_revision <= capsule.source_revision
                || after.ownership_status != "inactive"
                || singleton
                    != (
                        capsule.source_store_id.clone(),
                        capsule.receipt_sha256.clone(),
                        witness.handoff_id.clone(),
                        to_i64(capsule.source_revision)?,
                    )
                || latest
                    != (
                        capsule.source_store_id.clone(),
                        after.store_id.clone(),
                        to_i64(capsule.source_revision)?,
                        capsule.receipt_sha256.clone(),
                        witness.handoff_id.clone(),
                        capsule.receipt_sha256.clone(),
                    )
            {
                return Err(StoreError::ClaimConflict);
            }
            store
                .connection
                .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .map_err(|_| StoreError::Unavailable)?;
        }
        sync_dir(&stage)?;
        #[cfg(test)]
        if std::env::var("LIFECYCLE_HELD_TEST_CUT").as_deref() == Ok("adopt_before_publish") {
            std::process::exit(90);
        }
        publish_held_stage(&stage, destination)?;
        #[cfg(test)]
        if std::env::var("LIFECYCLE_HELD_TEST_CUT").as_deref() == Ok("adopt_after_publish") {
            std::process::exit(89);
        }
        fs::remove_file(crate::sqlite::adoption_intent_path(&stage)?)
            .map_err(|_| StoreError::Unavailable)?;
        sync_dir(destination.parent().ok_or(StoreError::Rejected)?)?;
        let store = Self::open(destination, trusted_issuer)?;
        store.held_projection()?;
        Ok(store)
    }
}

#[cfg(test)]
impl HeldSourceWitness {
    fn synthetic(handoff: &str, capture: &str, capsule: &str) -> Self {
        Self {
            handoff_id: handoff.into(),
            source_capture_sha256: capture.into(),
            capsule_sha256: capsule.into(),
            _sealed: (),
        }
    }
}

#[cfg(test)]
mod held_tests {
    use super::*;
    use serde_json::{Value, json};

    fn synthetic_evidence() -> HeldEvidence {
        let mut rows = Vec::new();
        let unresolved_tables = [
            ("active_turn_lifecycle", 81usize),
            ("context_input_admissions", 5),
            ("context_input_queue", 3),
            ("aborted_context_input_releases", 3),
            ("context_input_admission_handoffs", 2),
            ("checkpoint_fences", 5),
            ("checkpoint_publications", 1),
            ("lifecycle_ledger_heads", 1),
            ("role-bindings.json/bindings", 2),
            ("role-bindings.json/deliveries", 5),
        ];
        assert_eq!(unresolved_tables.iter().map(|(_, n)| n).sum::<usize>(), 108);
        for ordinal in 0..274 {
            let class = if ordinal == 0 {
                "terminally_retired_no_replay"
            } else if ordinal == 1 {
                "pending_accepted_undelivered"
            } else if ordinal < 110 {
                "unclassified"
            } else {
                "historical_observation"
            };
            let identity = format!("synthetic-row-{ordinal}");
            let table = if ordinal < 2 {
                "context_input_queue"
            } else if ordinal < 110 {
                let mut offset = ordinal - 2;
                let mut selected = "active_turn_lifecycle";
                for (name, n) in unresolved_tables {
                    if offset < n {
                        selected = name;
                        break;
                    }
                    offset -= n;
                }
                selected
            } else {
                "lifecycle_episodes"
            };
            rows.push(json!({
                "table":table, "identity":identity,
                "revision":format!("synthetic-revision-{ordinal}"),
                "status":"synthetic", "reason":if class=="unclassified" {Some("effect unresolved")} else {None},
                "source_projection_sha256":digest(format!("projection:{ordinal}").as_bytes()).unwrap(),
                "classification":class,
            }));
        }
        let inventory = Value::Array(rows.clone());
        let listed: Vec<Value> = rows
            .iter()
            .map(|row| {
                json!({
                    "table":row["table"],
                    "identity_sha256":digest(row["identity"].as_str().unwrap().as_bytes()).unwrap(),
                    "source_projection_sha256":row["source_projection_sha256"],
                    "classification":row["classification"],
                })
            })
            .collect();
        let canonical = serde_json::to_string(&inventory).unwrap();
        let mut capsule = json!({
            "schema_version":"s9-v28-obligations-v2",
            "translation_status":"qualified",
            "transfer_eligibility":"blocked_unclassified",
            "selection_claim":"current_selection_unestablished",
            "relation_census":(0..28).map(|n| format!("synthetic-relation-{n}")).collect::<Vec<_>>(),
            "inventory_digest_sha256":digest(canonical.as_bytes()).unwrap(),
            "row_classification_digests":listed,
            "q3":{"physical_delivery":"not_applicable"},
            "q4":{"physical_delivery":"not_proved"},
        });
        let mut artifacts = HELD_ARTIFACT_NAMES
            .iter()
            .map(|name| HeldArtifactInput {
                name: (*name).into(),
                raw_bytes: format!("synthetic exact bytes: {name}").into_bytes(),
            })
            .collect::<Vec<_>>();
        fn put(artifacts: &mut [HeldArtifactInput], name: &str, bytes: Vec<u8>) {
            artifacts
                .iter_mut()
                .find(|a| a.name == name)
                .unwrap()
                .raw_bytes = bytes;
        }
        // Store-mechanics fixture only. The separate A integration fixture
        // below proves canonical schema/codec translation through bootstrap.
        let db_root = tempfile::tempdir().unwrap();
        let db = db_root.path().join("synthetic.sqlite3");
        let connection = rusqlite::Connection::open(&db).unwrap();
        connection
            .execute_batch("CREATE TABLE synthetic(x INTEGER); PRAGMA user_version=12;")
            .unwrap();
        drop(connection);
        put(
            &mut artifacts,
            "source/capture/image/semantic-context.sqlite3",
            fs::read(&db).unwrap(),
        );
        for name in [
            "role-bindings.json",
            "generation-state.json",
            "switchboard-attachment.json",
        ] {
            put(
                &mut artifacts,
                &format!("source/capture/image/{name}"),
                b"{}".to_vec(),
            );
        }
        let image_names = [
            "semantic-context.sqlite3",
            "role-bindings.json",
            "generation-state.json",
            "switchboard-attachment.json",
        ];
        let get = |name: &str, values: &[HeldArtifactInput]| {
            values
                .iter()
                .find(|a| a.name == name)
                .unwrap()
                .raw_bytes
                .clone()
        };
        let mut source_entries = Vec::new();
        let mut attempt_artifacts = serde_json::Map::new();
        let mut image_hashes = serde_json::Map::new();
        for name in image_names {
            let bytes = get(&format!("source/capture/image/{name}"), &artifacts);
            let hash = digest(&bytes).unwrap();
            source_entries.push(json!({"copy":format!("/synthetic-b1/capture/image/{name}"),"copy_sha256":hash,"sha256":hash,"source":format!("/synthetic-b1/source/{name}"),"identity":{}}));
            attempt_artifacts.insert(format!("capture/image/{name}"), json!({"sha256":hash}));
            image_hashes.insert(name.into(), json!(hash));
        }
        let source_manifest = json!({"schema_version":1,"selection_claim":"current_selection_unestablished","files":source_entries});
        let source_manifest_bytes = serde_json::to_vec(&source_manifest).unwrap();
        put(
            &mut artifacts,
            "source/capture/source-manifest.json",
            source_manifest_bytes.clone(),
        );
        let child_receipt = json!({"schema_version":1,"status":"qualified_candidate_root","selection_claim":"current_selection_unestablished","manifest_sha256":digest(&source_manifest_bytes).unwrap()});
        let child_receipt_bytes = serde_json::to_vec(&child_receipt).unwrap();
        put(
            &mut artifacts,
            "source/capture/receipt.json",
            child_receipt_bytes.clone(),
        );
        let attempt = json!({"schema_version":1,"status":"qualified_candidate_root","selection_claim":"current_selection_unestablished","child_manifest_sha256":digest(&source_manifest_bytes).unwrap(),"child_receipt":{"sha256":digest(&child_receipt_bytes).unwrap()},"artifacts":attempt_artifacts});
        let attempt_bytes = serde_json::to_vec(&attempt).unwrap();
        put(
            &mut artifacts,
            "source/attempt-receipt.json",
            attempt_bytes.clone(),
        );
        capsule["source_attempt_sha256"] = json!(digest(&attempt_bytes).unwrap());
        capsule["source_child_receipt_sha256"] = json!(digest(&child_receipt_bytes).unwrap());
        capsule["source_manifest_sha256"] = json!(digest(&source_manifest_bytes).unwrap());
        capsule["copied_image_sha256"] = json!(image_hashes);
        for label in ["q3", "q4"] {
            let input = json!({"inputRevision":format!("input-{label}"),"logicalRoleInstanceId":format!("role-{label}"),"clientUserMessageId":format!("message-{label}"),"bindingRevision":1});
            let input_bytes = serde_json::to_vec(&input).unwrap();
            let successor = if label == "q3" {
                serde_json::Value::Null
            } else {
                json!({"acceptanceRevision":"accept-q4","successorLogicalRoleInstanceId":"role-successor-q4","successorDeliveryIdentity":"delivery-q4"})
            };
            let disposition = json!({"target":{"queueId":format!("queue-{label}"),"inputRevision":input["inputRevision"],"logicalRoleInstanceId":input["logicalRoleInstanceId"],"clientUserMessageId":input["clientUserMessageId"],"bindingRevision":input["bindingRevision"],"inputJsonSha256":digest(&input_bytes).unwrap()},"recordRevision":format!("disposition-{label}"),"authorityRevision":format!("authority-{label}"),"successorAcceptance":successor});
            put(
                &mut artifacts,
                &format!("output/raw/{label}-input.json"),
                input_bytes,
            );
            put(
                &mut artifacts,
                &format!("output/raw/{label}-disposition.json"),
                serde_json::to_vec(&disposition).unwrap(),
            );
            capsule[label] = json!({"physical_delivery":if label=="q3" {"not_applicable"} else {"not_proved"},"queue_id":disposition["target"]["queueId"],"input_revision":input["inputRevision"],"disposition_revision":disposition["recordRevision"],"authority_revision":disposition["authorityRevision"],"successor_acceptance_revision":if label=="q3" {serde_json::Value::Null} else {json!("accept-q4")},"successor_logical_role_instance_id":if label=="q3" {serde_json::Value::Null} else {json!("role-successor-q4")},"successor_delivery_identity":if label=="q3" {serde_json::Value::Null} else {json!("delivery-q4")}});
        }
        artifacts
            .iter_mut()
            .find(|a| a.name == "output/inventory-private.json")
            .unwrap()
            .raw_bytes = serde_json::to_vec(&inventory).unwrap();
        artifacts
            .iter_mut()
            .find(|a| a.name == "output/capsule.json")
            .unwrap()
            .raw_bytes = serde_json::to_vec(&capsule).unwrap();
        let mut files = artifacts
            .iter()
            .filter(|a| {
                a.name.starts_with("output/")
                    && a.name != "output/manifest.json"
                    && a.name != "output/receipt.json"
            })
            .map(|a| {
                json!({"path":a.name.strip_prefix("output/").unwrap(),
                "length":a.raw_bytes.len(),"sha256":digest(&a.raw_bytes).unwrap()})
            })
            .collect::<Vec<_>>();
        files.sort_by(|a, b| a["path"].as_str().cmp(&b["path"].as_str()));
        let manifest = json!({"schema_version":1,"files":files});
        artifacts
            .iter_mut()
            .find(|a| a.name == "output/manifest.json")
            .unwrap()
            .raw_bytes = serde_json::to_vec(&manifest).unwrap();
        let get = |name: &str| {
            artifacts
                .iter()
                .find(|a| a.name == name)
                .unwrap()
                .raw_bytes
                .clone()
        };
        let receipt = json!({
            "schema_version":1,"capsule_version":"s9-v28-obligations-v2",
            "status":"qualified_translation_only","transfer_eligibility":"blocked_unclassified",
            "source_attempt_sha256":digest(&get("source/attempt-receipt.json")).unwrap(),
            "source_manifest_sha256":digest(&get("source/capture/source-manifest.json")).unwrap(),
            "source_child_receipt_sha256":digest(&get("source/capture/receipt.json")).unwrap(),
            "manifest_sha256":digest(&get("output/manifest.json")).unwrap(),
            "capsule_sha256":digest(&get("output/capsule.json")).unwrap(),
        });
        artifacts
            .iter_mut()
            .find(|a| a.name == "output/receipt.json")
            .unwrap()
            .raw_bytes = serde_json::to_vec(&receipt).unwrap();
        HeldEvidence { artifacts }
    }

    fn witness(evidence: &HeldEvidence) -> HeldSourceWitness {
        let (_, _, capture, capsule, _, _) = validate_held_evidence(evidence).unwrap();
        HeldSourceWitness::synthetic("handoff:synthetic-b1", &capture, &capsule)
    }

    #[test]
    fn held_bootstrap_crash_cut_child() {
        let Ok(destination) = std::env::var("LIFECYCLE_HELD_TEST_DEST") else {
            return;
        };
        let evidence = synthetic_evidence();
        let witness = witness(&evidence);
        let _ = SqliteLifecycleStore::bootstrap_held(
            &witness,
            &evidence,
            Path::new(&destination),
            "issuer".into(),
        );
        panic!("requested crash cut did not stop child");
    }

    #[test]
    fn held_bootstrap_exact_retry_after_both_publish_cuts() {
        let temp = tempfile::tempdir().unwrap();
        for (phase, code) in [
            ("after_reservation", 84),
            ("after_stage_open", 83),
            ("before_publish", 81),
            ("after_publish", 82),
        ] {
            let destination = temp.path().join(format!("dest-{phase}"));
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .arg("--exact")
                .arg("admin::held_tests::held_bootstrap_crash_cut_child")
                .env("LIFECYCLE_HELD_TEST_DEST", &destination)
                .env("LIFECYCLE_HELD_TEST_CUT", phase)
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(code));
            assert_eq!(destination.exists(), phase == "after_publish");
            let reservation: HeldDestinationReservation = serde_json::from_slice(
                &fs::read(held_destination_reservation_path(&destination).unwrap()).unwrap(),
            )
            .unwrap();
            let original_id = if phase == "after_reservation" {
                assert!(!reservation.stage.exists());
                None
            } else {
                let recorded_root = if phase == "after_publish" {
                    &destination
                } else {
                    &reservation.stage
                };
                let inspected = rusqlite::Connection::open_with_flags(
                    recorded_root.join("lifecycle.sqlite"),
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                )
                .unwrap();
                let id: String = inspected
                    .query_row(
                        "SELECT store_id FROM store_identity WHERE singleton=1",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                Some(id)
            };
            if phase != "after_publish" && phase != "after_reservation" {
                assert!(SqliteLifecycleStore::open(&reservation.stage, "issuer".into()).is_err());
            }
            let evidence = synthetic_evidence();
            let witness = witness(&evidence);
            let mut changed_evidence = evidence.clone();
            changed_evidence
                .artifacts
                .iter_mut()
                .find(|a| a.name == "output/report-redacted.json")
                .unwrap()
                .raw_bytes = b"{}".to_vec();
            let mut changed_manifest: Value = serde_json::from_slice(
                &changed_evidence
                    .artifacts
                    .iter()
                    .find(|a| a.name == "output/manifest.json")
                    .unwrap()
                    .raw_bytes,
            )
            .unwrap();
            let report = changed_manifest["files"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|entry| entry["path"] == "report-redacted.json")
                .unwrap();
            report["sha256"] = json!(digest(b"{}").unwrap());
            report["length"] = json!(2);
            let manifest_bytes = serde_json::to_vec(&changed_manifest).unwrap();
            changed_evidence
                .artifacts
                .iter_mut()
                .find(|a| a.name == "output/manifest.json")
                .unwrap()
                .raw_bytes = manifest_bytes.clone();
            let mut changed_receipt: Value = serde_json::from_slice(
                &changed_evidence
                    .artifacts
                    .iter()
                    .find(|a| a.name == "output/receipt.json")
                    .unwrap()
                    .raw_bytes,
            )
            .unwrap();
            changed_receipt["manifest_sha256"] = json!(digest(&manifest_bytes).unwrap());
            changed_evidence
                .artifacts
                .iter_mut()
                .find(|a| a.name == "output/receipt.json")
                .unwrap()
                .raw_bytes = serde_json::to_vec(&changed_receipt).unwrap();
            assert!(SqliteLifecycleStore::inspect_held_evidence(&changed_evidence).is_ok());
            let wrong = HeldSourceWitness::synthetic(
                "handoff:conflict",
                &witness.source_capture_sha256,
                &witness.capsule_sha256,
            );
            assert!(matches!(
                SqliteLifecycleStore::bootstrap_held(
                    &wrong,
                    &evidence,
                    &destination,
                    "issuer".into()
                ),
                Err(StoreError::ClaimConflict)
            ));
            assert!(matches!(
                SqliteLifecycleStore::bootstrap_held(
                    &witness,
                    &evidence,
                    &destination,
                    "different-issuer".into()
                ),
                Err(StoreError::ClaimConflict)
            ));
            assert!(matches!(
                SqliteLifecycleStore::bootstrap_held(
                    &witness,
                    &changed_evidence,
                    &destination,
                    "issuer".into()
                ),
                Err(StoreError::ClaimConflict)
            ));
            let store = SqliteLifecycleStore::bootstrap_held(
                &witness,
                &evidence,
                &destination,
                "issuer".into(),
            )
            .unwrap();
            let projection = store.held_projection().unwrap();
            if let Some(original_id) = original_id {
                assert_eq!(projection.store_id, original_id);
            }
            assert_eq!(
                (projection.row_count, projection.unclassified_count),
                (274, 108)
            );
            assert_eq!(projection.ownership_status, "inactive");
        }
    }

    #[test]
    fn held_adoption_crash_cut_child() {
        let Ok(destination) = std::env::var("LIFECYCLE_HELD_TEST_DEST") else {
            return;
        };
        let capsule_root = PathBuf::from(std::env::var("LIFECYCLE_HELD_TEST_CAPSULE").unwrap());
        let capsule = SqliteLifecycleStore::read_held_capsule(&capsule_root).unwrap();
        let witness = HeldSourceWitness::synthetic(
            &capsule.handoff_id,
            &capsule.source_capture_sha256,
            &capsule.origin_capsule_sha256,
        );
        let _ = SqliteLifecycleStore::adopt_held_inactive(
            &witness,
            &capsule,
            Path::new(&destination),
            "issuer".into(),
        );
        panic!("requested adoption crash cut did not stop child");
    }

    #[test]
    fn held_adoption_exact_retry_all_crash_cuts_and_cross_operation_conflicts() {
        let temp = tempfile::tempdir().unwrap();
        let evidence = synthetic_evidence();
        let witness = witness(&evidence);
        let source_root = temp.path().join("source");
        let source = SqliteLifecycleStore::bootstrap_held(
            &witness,
            &evidence,
            &source_root,
            "issuer".into(),
        )
        .unwrap();
        let capsule = source.export_held(&temp.path().join("capsule-v1")).unwrap();
        source
            .connection
            .execute(
                "UPDATE admin_ownership SET revision=revision+1 WHERE singleton=1 AND status='inactive'",
                [],
            )
            .unwrap();
        let changed_capsule = source.export_held(&temp.path().join("capsule-v2")).unwrap();
        assert_ne!(capsule.receipt_sha256, changed_capsule.receipt_sha256);
        assert_ne!(capsule.source_revision, changed_capsule.source_revision);
        drop(source);
        for (phase, code) in [
            ("adopt_after_reservation", 94),
            ("adopt_after_copy", 93),
            ("adopt_after_stage_open", 92),
            ("adopt_after_lineage_commit", 91),
            ("adopt_before_publish", 90),
            ("adopt_after_publish", 89),
        ] {
            let destination = temp.path().join(format!("adopt-{phase}"));
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .arg("--exact")
                .arg("admin::held_tests::held_adoption_crash_cut_child")
                .env("LIFECYCLE_HELD_TEST_DEST", &destination)
                .env("LIFECYCLE_HELD_TEST_CAPSULE", &capsule.capsule_root)
                .env("LIFECYCLE_HELD_TEST_CUT", phase)
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(code), "cut {phase}");
            assert_eq!(destination.exists(), phase == "adopt_after_publish");
            let reservation: HeldDestinationReservation = serde_json::from_slice(
                &fs::read(held_destination_reservation_path(&destination).unwrap()).unwrap(),
            )
            .unwrap();
            assert_eq!(reservation.destination, destination);
            assert_eq!(reservation.handoff_id, witness.handoff_id);
            assert_eq!(reservation.trusted_issuer, "issuer");
            assert_eq!(
                reservation.operation,
                HeldReservationOperation::Adopt {
                    receipt_sha256: capsule.receipt_sha256.clone(),
                    source_store_id: capsule.source_store_id.clone(),
                    source_revision: capsule.source_revision,
                    source_custody_phase: capsule.source_custody_phase.clone(),
                    source_database_sha256: capsule.database_sha256.clone(),
                    capsule_root: capsule.capsule_root.clone(),
                }
            );
            let original_id = if phase == "adopt_after_reservation" {
                assert!(!reservation.stage.exists());
                ensure_held_destination_stage(&reservation.stage).unwrap();
                fs::write(
                    reservation.stage.join(".lifecycle.sqlite.copying"),
                    b"partial copy",
                )
                .unwrap();
                None
            } else {
                let recorded_root = if phase == "adopt_after_publish" {
                    &destination
                } else {
                    &reservation.stage
                };
                let conn = rusqlite::Connection::open_with_flags(
                    recorded_root.join("lifecycle.sqlite"),
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                )
                .unwrap();
                let id: String = conn
                    .query_row(
                        "SELECT store_id FROM store_identity WHERE singleton=1",
                        [],
                        |r| r.get(0),
                    )
                    .unwrap();
                if phase != "adopt_after_publish" {
                    assert!(
                        SqliteLifecycleStore::open(&reservation.stage, "issuer".into()).is_err()
                    );
                }
                Some(id)
            };
            assert!(matches!(
                SqliteLifecycleStore::adopt_held_inactive(
                    &witness,
                    &changed_capsule,
                    &destination,
                    "issuer".into(),
                ),
                Err(StoreError::ClaimConflict)
            ));
            assert!(matches!(
                SqliteLifecycleStore::adopt_held_inactive(
                    &witness,
                    &capsule,
                    &destination,
                    "other-issuer".into(),
                ),
                Err(StoreError::ClaimConflict)
            ));
            assert!(matches!(
                SqliteLifecycleStore::bootstrap_held(
                    &witness,
                    &evidence,
                    &destination,
                    "issuer".into(),
                ),
                Err(StoreError::ClaimConflict)
            ));
            let adopted = SqliteLifecycleStore::adopt_held_inactive(
                &witness,
                &capsule,
                &destination,
                "issuer".into(),
            )
            .unwrap();
            let projection = adopted.held_projection().unwrap();
            assert_eq!(projection.ownership_status, "inactive");
            assert_eq!(projection.row_count, 274);
            assert_eq!(projection.unclassified_count, 108);
            if let Some(original_id) = original_id
                && phase != "adopt_after_copy"
                && phase != "adopt_after_stage_open"
            {
                assert_eq!(projection.store_id, original_id);
            }
            let lineages: i64 = adopted
                .connection
                .query_row("SELECT COUNT(*) FROM legacy_held_lineage", [], |r| r.get(0))
                .unwrap();
            assert_eq!(lineages, 1);
            drop(adopted);
            assert_eq!(
                SqliteLifecycleStore::adopt_held_inactive(
                    &witness,
                    &capsule,
                    &destination,
                    "issuer".into(),
                )
                .unwrap()
                .held_projection()
                .unwrap(),
                projection
            );
            assert!(!reservation.stage.exists());
            assert!(
                !crate::sqlite::adoption_intent_path(&reservation.stage)
                    .unwrap()
                    .exists()
            );
        }
        let reserved_bootstrap = temp.path().join("bootstrap-before-adopt");
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("admin::held_tests::held_bootstrap_crash_cut_child")
            .env("LIFECYCLE_HELD_TEST_DEST", &reserved_bootstrap)
            .env("LIFECYCLE_HELD_TEST_CUT", "after_reservation")
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(84));
        assert!(matches!(
            SqliteLifecycleStore::adopt_held_inactive(
                &witness,
                &capsule,
                &reserved_bootstrap,
                "issuer".into(),
            ),
            Err(StoreError::ClaimConflict)
        ));
        assert!(!reserved_bootstrap.exists());
    }

    #[test]
    fn held_evidence_tamper_and_conflicting_retry_refuse() {
        let evidence = synthetic_evidence();
        let witness = witness(&evidence);
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("target");
        let prepopulated = temp.path().join("prepopulated");
        fs::create_dir(&prepopulated).unwrap();
        assert!(
            SqliteLifecycleStore::bootstrap_held(
                &witness,
                &evidence,
                &prepopulated,
                "issuer".into()
            )
            .is_err()
        );
        assert_eq!(fs::read_dir(&prepopulated).unwrap().count(), 0);
        let mut tampered = evidence.clone();
        tampered
            .artifacts
            .iter_mut()
            .find(|a| a.name == "output/inventory-private.json")
            .unwrap()
            .raw_bytes
            .push(b' ');
        assert!(SqliteLifecycleStore::inspect_held_evidence(&tampered).is_err());
        assert!(
            SqliteLifecycleStore::bootstrap_held(
                &witness,
                &tampered,
                &destination,
                "issuer".into()
            )
            .is_err()
        );
        assert!(!destination.exists());
        let mut missing = evidence.clone();
        missing
            .artifacts
            .retain(|a| a.name != "output/raw/q4-input.json");
        assert!(SqliteLifecycleStore::inspect_held_evidence(&missing).is_err());
        let store = SqliteLifecycleStore::bootstrap_held(
            &witness,
            &evidence,
            &destination,
            "issuer".into(),
        )
        .unwrap();
        drop(store);
        let alias = temp.path().join("alias");
        std::os::unix::fs::symlink(&destination, &alias).unwrap();
        assert!(
            SqliteLifecycleStore::bootstrap_held(&witness, &evidence, &alias, "issuer".into())
                .is_err()
        );
        let conflict = HeldSourceWitness::synthetic(
            "handoff:other",
            &witness.source_capture_sha256,
            &witness.capsule_sha256,
        );
        assert!(matches!(
            SqliteLifecycleStore::bootstrap_held(
                &conflict,
                &evidence,
                &destination,
                "issuer".into()
            ),
            Err(StoreError::ClaimConflict)
        ));
    }

    #[test]
    fn synthetic_274_108_bootstrap_quarantine_export_and_readopt() {
        let temp = tempfile::tempdir().unwrap();
        let evidence = synthetic_evidence();
        let witness = witness(&evidence);
        let target = temp.path().join("absent-held-target");
        assert!(!target.exists());
        let mut store =
            SqliteLifecycleStore::bootstrap_held(&witness, &evidence, &target, "issuer".into())
                .unwrap();
        let first = store.held_projection().unwrap();
        assert_eq!((first.row_count, first.unclassified_count), (274, 108));
        assert_eq!(first.ownership_status, "inactive");
        assert_eq!(first.artifact_digests.len(), 16);
        assert!(!admission_open(&store.connection).unwrap());
        use lifecycle_core::{
            AuthorityExpiresAt, BuildId, ClockSample, ContextGeneration, GrantId, GrantScope,
            ProofRunId, Revision, SubjectId, TrustedGrant, WaitBudgetMs, WallTimeMs,
        };
        let subject = SubjectId::parse("brand-new-ordinary-subject").unwrap();
        let context = ContextGeneration::parse("brand-new-context").unwrap();
        let build = BuildId::parse("brand-new-build").unwrap();
        let proof = ProofRunId::parse("brand-new-proof").unwrap();
        assert!(
            store
                .register_subject(
                    subject.clone(),
                    context.clone(),
                    build.clone(),
                    proof.clone(),
                    ClockSample {
                        wall: WallTimeMs::new(10),
                        wait_budget: WaitBudgetMs::new(10),
                    }
                )
                .is_err()
        );
        assert!(
            store
                .install_trusted_grant(TrustedGrant {
                    id: GrantId::parse("brand-new-valid-grant").unwrap(),
                    issuer: "issuer".into(),
                    principal: "operator:synthetic".into(),
                    subject,
                    context,
                    build,
                    proof_run: proof,
                    scope: GrantScope::EnqueueInput,
                    expires: AuthorityExpiresAt::new(WallTimeMs::new(1000)),
                    revision: Revision::new(1),
                    revoked: false,
                })
                .is_err()
        );
        assert!(store.connection.execute("INSERT INTO subjects(subject_id,context_id,build_id,proof_run_id,revision,semantic_revision,queue_revision,owner_kind,last_wall_ms) VALUES ('sql-new-id','ctx','build','proof',0,0,0,'domain',10)",[]).is_err());
        assert!(store.connection.execute("INSERT INTO legacy_held_artifacts(name,sha256,byte_length,raw_bytes) VALUES ('late','00',0,X'')",[]).is_err());
        assert!(store.connection.execute("INSERT INTO legacy_held_rows(ordinal,table_name,source_identity,source_revision,source_projection_sha256,classification,row_json) VALUES (274,'late','late',NULL,'00','unclassified',X'7B7D')",[]).is_err());
        assert!(
            store
                .export_current(&temp.path().join("ordinary-refused"))
                .is_err()
        );
        assert!(!temp.path().join("ordinary-refused").exists());
        assert!(
            store
                .commit_destination(
                    &lifecycle_core::HandoffId::parse("handoff:synthetic-b1").unwrap(),
                    &FenceReceipt {
                        handoff_id: "bad".into(),
                        source_store_id: "bad".into(),
                        source_root: temp.path().join("bad"),
                        destination_store_id: "bad".into(),
                        capsule_sha256: "bad".into(),
                        source_revision: 0
                    },
                    &SelectionEvidence {
                        handoff_id: "bad".into(),
                        source_store_id: "bad".into(),
                        destination_store_id: "bad".into(),
                        capsule_sha256: "bad".into(),
                        prior_selection_revision: 0,
                        current_selection_revision: 0,
                        current_selection_digest: "bad".into(),
                        launch_guard_id: "bad".into()
                    }
                )
                .is_err()
        );
        let capsule_root = temp.path().join("held-export");
        let receipt = store.export_held(&capsule_root).unwrap();
        assert_eq!(
            SqliteLifecycleStore::read_held_capsule(&capsule_root).unwrap(),
            receipt
        );
        assert!(SqliteLifecycleStore::read_capsule(&capsule_root).is_err());
        drop(store);
        let reopened = SqliteLifecycleStore::open(&target, "issuer".into()).unwrap();
        assert_eq!(reopened.held_projection().unwrap(), first);
        drop(reopened);
        let retry =
            SqliteLifecycleStore::bootstrap_held(&witness, &evidence, &target, "issuer".into())
                .unwrap();
        assert_eq!(retry.held_projection().unwrap(), first);
        drop(retry);
        let adopted_root = temp.path().join("readopt-held-target");
        let adopted = SqliteLifecycleStore::adopt_held_inactive(
            &witness,
            &receipt,
            &adopted_root,
            "issuer".into(),
        )
        .unwrap();
        let after = adopted.held_projection().unwrap();
        assert_eq!((after.row_count, after.unclassified_count), (274, 108));
        assert_eq!(after.artifact_digests, first.artifact_digests);
        assert!(after.custody_revision > first.custody_revision);
        assert_ne!(after.store_id, first.store_id);
        assert_eq!(after.ownership_status, "inactive");
        let second_capsule = adopted
            .export_held(&temp.path().join("held-export-from-b"))
            .unwrap();
        drop(adopted);
        let retry_adopt = SqliteLifecycleStore::adopt_held_inactive(
            &witness,
            &receipt,
            &adopted_root,
            "issuer".into(),
        )
        .unwrap();
        assert_eq!(retry_adopt.held_projection().unwrap(), after);
        drop(retry_adopt);
        let third_root = temp.path().join("held-third-root");
        let third = SqliteLifecycleStore::adopt_held_inactive(
            &witness,
            &second_capsule,
            &third_root,
            "issuer".into(),
        )
        .unwrap();
        let third_projection = third.held_projection().unwrap();
        assert_ne!(third_projection.store_id, after.store_id);
        assert!(third_projection.custody_revision > after.custody_revision);
        assert_eq!(third_projection.artifact_digests, first.artifact_digests);
        assert_eq!(third_projection.ownership_status, "inactive");
        assert_eq!(
            third
                .connection
                .query_row("SELECT COUNT(*) FROM legacy_held_lineage", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            2
        );
        let predecessor: String = third
            .connection
            .query_row(
                "SELECT source_store_id FROM admin_store_lineage WHERE singleton=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(predecessor, after.store_id);
        drop(third);
        let third_retry = SqliteLifecycleStore::adopt_held_inactive(
            &witness,
            &second_capsule,
            &third_root,
            "issuer".into(),
        )
        .unwrap();
        assert_eq!(third_retry.held_projection().unwrap(), third_projection);
        drop(third_retry);
        // Synthetic recovery-only phase: the held marker still vetoes normal
        // admission even if a future selected owner has active admin status.
        let source = SqliteLifecycleStore::open(&target, "issuer".into()).unwrap();
        source.connection.execute("UPDATE admin_ownership SET status='active',revision=revision+1 WHERE singleton=1 AND status='inactive'",[]).unwrap();
        assert!(!admission_open(&source.connection).unwrap());
        let active_capsule_root = temp.path().join("recovery-only-export");
        let active_receipt = source.export_held(&active_capsule_root).unwrap();
        assert_eq!(active_receipt.source_custody_phase, "recovery_only");
        drop(source);
        let forward_root = temp.path().join("recovery-only-readopt");
        let forward = SqliteLifecycleStore::adopt_held_inactive(
            &witness,
            &active_receipt,
            &forward_root,
            "issuer".into(),
        )
        .unwrap();
        assert_eq!(
            forward.held_projection().unwrap().ownership_status,
            "inactive"
        );
    }

    #[test]
    #[ignore = "requires S9_B1_SYNTHETIC_CLOSURE_ROOT produced by pinned native capture and production A-v2 readback"]
    fn verified_native_a_v2_closure_bootstraps_same_held_store_path() {
        let root = PathBuf::from(std::env::var("S9_B1_SYNTHETIC_CLOSURE_ROOT").unwrap());
        let evidence = HeldEvidence {
            artifacts: HELD_ARTIFACT_NAMES
                .iter()
                .map(|name| HeldArtifactInput {
                    name: (*name).into(),
                    raw_bytes: fs::read(root.join(name)).unwrap(),
                })
                .collect(),
        };
        assert_eq!(
            SqliteLifecycleStore::inspect_held_evidence(&evidence).unwrap(),
            (274, 108)
        );
        let witness = witness(&evidence);
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("native-a-v2-held");
        let source = temp.path().join("synthetic-legacy-source");
        fs::create_dir(&source).unwrap();
        let selector_root = temp.path().join("synthetic-selector");
        let legacy = crate::held_selection::LegacySelection {
            version: 1,
            generation: 0,
            root: source,
            owner_id: "synthetic-v28-owner".into(),
            owner_revision: "synthetic-v28-revision".into(),
            executable_sha256: digest(b"synthetic-legacy-executable").unwrap(),
            config_sha256: digest(b"synthetic-legacy-config").unwrap(),
        };
        let mut spec = crate::held_selection::CutSpec {
            predecessor: legacy.clone(),
            handoff_id: witness.handoff_id.clone(),
            destination_root: destination.clone(),
            capture_sha256: witness.source_capture_sha256.clone(),
            capsule_sha256: witness.capsule_sha256.clone(),
            source_fence_sha256: digest(b"synthetic-excluded-source-fence").unwrap(),
            build_sha256: digest(b"synthetic-recovery-only-build").unwrap(),
            config_sha256: digest(b"synthetic-recovery-only-config").unwrap(),
        };
        let selector =
            crate::held_selection::HeldCutSession::synthetic_initialize(&selector_root, &legacy)
                .unwrap();
        selector.reserve(spec.clone()).unwrap();
        selector.reserve(spec.clone()).unwrap();
        spec.handoff_id = "handoff:conflicting-synthetic".into();
        assert!(selector.reserve(spec.clone()).is_err());
        spec.handoff_id = witness.handoff_id.clone();
        drop(selector);
        let selector =
            crate::held_selection::HeldCutSession::synthetic_resume(&selector_root).unwrap();
        selector.reserve(spec.clone()).unwrap();
        let mut changed_before_bootstrap = evidence.clone();
        changed_before_bootstrap
            .artifacts
            .iter_mut()
            .find(|a| a.name == "source/capture/image/role-bindings.json")
            .unwrap()
            .raw_bytes
            .push(b' ');
        assert!(
            SqliteLifecycleStore::bootstrap_held(
                &witness,
                &changed_before_bootstrap,
                &destination,
                "issuer".into(),
            )
            .is_err()
        );
        assert!(!destination.exists());
        let store = SqliteLifecycleStore::bootstrap_held(
            &witness,
            &evidence,
            &destination,
            "issuer".into(),
        )
        .unwrap();
        let projection = store.held_projection().unwrap();
        assert_eq!(
            (projection.row_count, projection.unclassified_count),
            (274, 108)
        );
        assert_eq!(projection.ownership_status, "inactive");
        assert_eq!(projection.artifact_digests.len(), 16);
        assert!(!admission_open(&store.connection).unwrap());
        assert!(selector.synthetic_select(&store, false).is_err());
        assert!(
            selector
                .synthetic_fence(&digest(b"wrong-fence").unwrap())
                .is_err()
        );
        selector.synthetic_fence(&spec.source_fence_sha256).unwrap();
        selector.synthetic_fence(&spec.source_fence_sha256).unwrap();
        drop(selector);
        let selector =
            crate::held_selection::HeldCutSession::synthetic_resume(&selector_root).unwrap();
        assert!(selector.synthetic_select(&store, true).is_err());
        drop(selector);
        let selector =
            crate::held_selection::HeldCutSession::synthetic_resume(&selector_root).unwrap();
        let selected = selector.synthetic_select(&store, false).unwrap();
        assert_eq!(selected.phase, "selected_recovery_only");
        assert_eq!(selected.destination_store_id, projection.store_id);
        assert_eq!(
            selected.destination_custody_revision,
            projection.custody_revision
        );
        assert_eq!(selector.verify_selected(&store).unwrap(), selected);
        selector.reserve(spec.clone()).unwrap();
        selector.synthetic_fence(&spec.source_fence_sha256).unwrap();
        assert_eq!(selector.synthetic_select(&store, false).unwrap(), selected);
        assert_eq!(store.admin_ownership().unwrap().status, "inactive");
        let held_capsule = temp.path().join("held-export");
        let receipt = store.export_held(&held_capsule).unwrap();
        assert_eq!(receipt.source_custody_phase, "inactive");
        assert_eq!(
            SqliteLifecycleStore::read_held_capsule(&held_capsule).unwrap(),
            receipt
        );
        let adopted = SqliteLifecycleStore::adopt_held_inactive(
            &witness,
            &receipt,
            &temp.path().join("held-adopted"),
            "issuer".into(),
        )
        .unwrap();
        assert_eq!(
            adopted.held_projection().unwrap().ownership_status,
            "inactive"
        );
        assert!(selector.verify_selected(&adopted).is_err());
        let stored_rows: i64 = store
            .connection
            .query_row("SELECT COUNT(*) FROM legacy_held_rows", [], |r| r.get(0))
            .unwrap();
        assert_eq!(stored_rows, 274);
        drop(store);
        let reopened = SqliteLifecycleStore::open(&destination, "issuer".into()).unwrap();
        assert_eq!(reopened.held_projection().unwrap(), projection);
        assert_eq!(selector.verify_selected(&reopened).unwrap(), selected);
        drop(reopened);
        drop(selector);
        let selector =
            crate::held_selection::HeldCutSession::synthetic_resume(&selector_root).unwrap();
        let reopened = SqliteLifecycleStore::open(&destination, "issuer".into()).unwrap();
        assert_eq!(selector.verify_selected(&reopened).unwrap(), selected);
        drop(reopened);
        for name in [
            "source/attempt-receipt.json",
            "source/capture/source-manifest.json",
            "source/capture/receipt.json",
            "source/capture/image/semantic-context.sqlite3",
            "source/capture/image/role-bindings.json",
            "source/capture/image/generation-state.json",
            "source/capture/image/switchboard-attachment.json",
        ] {
            let mut changed = evidence.clone();
            changed
                .artifacts
                .iter_mut()
                .find(|a| a.name == name)
                .unwrap()
                .raw_bytes
                .push(b' ');
            assert!(
                SqliteLifecycleStore::inspect_held_evidence(&changed).is_err(),
                "changed source member {name} passed"
            );
            assert!(
                SqliteLifecycleStore::bootstrap_held(
                    &witness,
                    &changed,
                    &temp.path().join("changed-source"),
                    "issuer".into()
                )
                .is_err()
            );
        }
        assert!(held_json(br#"{"x":1,"x":2}"#).is_err());
        assert!(held_json(b"{\xff}").is_err());
        let mut bad_schema = evidence.clone();
        let database = &mut bad_schema
            .artifacts
            .iter_mut()
            .find(|a| a.name == "source/capture/image/semantic-context.sqlite3")
            .unwrap()
            .raw_bytes;
        database[60..64].copy_from_slice(&11_u32.to_be_bytes());
        assert!(SqliteLifecycleStore::inspect_held_evidence(&bad_schema).is_err());
    }

    #[test]
    #[ignore = "requires S9_B1_SYNTHETIC_CLOSURE_ROOT produced by pinned native capture and production A-v2 readback"]
    fn verified_native_a_v2_selector_crash_boundary_matrix() {
        use crate::held_selection::{
            CutSpec, HeldCutSession, LegacySelection, WriteCut, WriteKind, arm_test_cut,
            clear_test_cut,
        };

        let root = PathBuf::from(std::env::var("S9_B1_SYNTHETIC_CLOSURE_ROOT").unwrap());
        let evidence = HeldEvidence {
            artifacts: HELD_ARTIFACT_NAMES
                .iter()
                .map(|name| HeldArtifactInput {
                    name: (*name).into(),
                    raw_bytes: fs::read(root.join(name)).unwrap(),
                })
                .collect(),
        };
        assert_eq!(
            SqliteLifecycleStore::inspect_held_evidence(&evidence).unwrap(),
            (274, 108)
        );
        let witness = witness(&evidence);
        let recoverable = [
            WriteCut::Write,
            WriteCut::FileSync,
            WriteCut::StageDirSync,
            WriteCut::Rename,
            WriteCut::PublishDirSync,
        ];
        for kind in [
            WriteKind::Reserve,
            WriteKind::Fence,
            WriteKind::Selector,
            WriteKind::SelectedIntent,
        ] {
            for cut in recoverable.into_iter().chain([WriteCut::Create]) {
                let temp = tempfile::tempdir().unwrap();
                let source = temp.path().join("synthetic-legacy-source");
                fs::create_dir(&source).unwrap();
                let destination = temp.path().join("native-a-v2-held");
                let selector_root = temp.path().join("synthetic-selector");
                let legacy = LegacySelection {
                    version: 1,
                    generation: 0,
                    root: source,
                    owner_id: "synthetic-v28-owner".into(),
                    owner_revision: "synthetic-v28-revision".into(),
                    executable_sha256: digest(b"synthetic-legacy-executable").unwrap(),
                    config_sha256: digest(b"synthetic-legacy-config").unwrap(),
                };
                let spec = CutSpec {
                    predecessor: legacy.clone(),
                    handoff_id: witness.handoff_id.clone(),
                    destination_root: destination.clone(),
                    capture_sha256: witness.source_capture_sha256.clone(),
                    capsule_sha256: witness.capsule_sha256.clone(),
                    source_fence_sha256: digest(b"synthetic-excluded-source-fence").unwrap(),
                    build_sha256: digest(b"synthetic-recovery-only-build").unwrap(),
                    config_sha256: digest(b"synthetic-recovery-only-config").unwrap(),
                };
                let selector =
                    HeldCutSession::synthetic_initialize(&selector_root, &legacy).unwrap();
                if kind == WriteKind::Reserve {
                    arm_test_cut(kind, cut);
                    assert!(selector.reserve(spec.clone()).is_err(), "{kind:?} {cut:?}");
                    clear_test_cut();
                    drop(selector);
                } else {
                    selector.reserve(spec.clone()).unwrap();
                    drop(selector);
                }
                let selector = HeldCutSession::synthetic_resume(&selector_root).unwrap();
                let mut conflict = spec.clone();
                conflict.handoff_id = "handoff:conflicting-synthetic".into();
                assert!(selector.reserve(conflict).is_err(), "{kind:?} {cut:?}");
                if kind == WriteKind::Reserve && cut == WriteCut::Create {
                    // An empty stage cannot establish an intent; it is held for
                    // explicit repair, and no B1 destination has been written.
                    assert!(selector.reserve(spec).is_err());
                    assert!(!destination.exists());
                    continue;
                }
                selector.reserve(spec.clone()).unwrap();
                let store = SqliteLifecycleStore::bootstrap_held(
                    &witness,
                    &evidence,
                    &destination,
                    "issuer".into(),
                )
                .unwrap();
                let projection = store.held_projection().unwrap();
                assert_eq!(
                    (projection.row_count, projection.unclassified_count),
                    (274, 108)
                );
                assert_eq!(projection.ownership_status, "inactive");
                assert_eq!(projection.artifact_digests.len(), 16);
                assert!(!admission_open(&store.connection).unwrap());

                if kind == WriteKind::Fence {
                    arm_test_cut(kind, cut);
                    assert!(selector.synthetic_fence(&spec.source_fence_sha256).is_err());
                    clear_test_cut();
                    drop(selector);
                } else {
                    selector.synthetic_fence(&spec.source_fence_sha256).unwrap();
                    drop(selector);
                }
                let selector = HeldCutSession::synthetic_resume(&selector_root).unwrap();
                let mut conflict = spec.clone();
                conflict.handoff_id = "handoff:conflicting-synthetic".into();
                assert!(selector.reserve(conflict).is_err(), "{kind:?} {cut:?}");
                if kind == WriteKind::Fence && cut == WriteCut::Create {
                    assert!(selector.synthetic_fence(&spec.source_fence_sha256).is_err());
                    assert!(selector.verify_selected(&store).is_err());
                    assert_eq!(store.held_projection().unwrap(), projection);
                    continue;
                }
                selector.synthetic_fence(&spec.source_fence_sha256).unwrap();

                if kind == WriteKind::Selector || kind == WriteKind::SelectedIntent {
                    arm_test_cut(kind, cut);
                    assert!(selector.synthetic_select(&store, false).is_err());
                    clear_test_cut();
                    drop(selector);
                } else {
                    drop(selector);
                }
                let selector = HeldCutSession::synthetic_resume(&selector_root).unwrap();
                let mut conflict = spec.clone();
                conflict.handoff_id = "handoff:conflicting-synthetic".into();
                assert!(selector.reserve(conflict).is_err(), "{kind:?} {cut:?}");
                if matches!(kind, WriteKind::Selector | WriteKind::SelectedIntent)
                    && cut == WriteCut::Create
                {
                    assert!(selector.synthetic_select(&store, false).is_err());
                    assert!(selector.verify_selected(&store).is_err());
                    assert_eq!(store.held_projection().unwrap(), projection);
                    continue;
                }
                drop(store);
                let reopened = SqliteLifecycleStore::open(&destination, "issuer".into()).unwrap();
                assert_eq!(reopened.held_projection().unwrap(), projection);
                assert!(!admission_open(&reopened.connection).unwrap());
                let selected = selector.synthetic_select(&reopened, false).unwrap();
                assert_eq!(selected.phase, "selected_recovery_only");
                assert_eq!(selected.destination_store_id, projection.store_id);
                assert_eq!(
                    selected.destination_custody_revision,
                    projection.custody_revision
                );
                assert_eq!(selector.verify_selected(&reopened).unwrap(), selected);
                drop(selector);
                let selector = HeldCutSession::synthetic_resume(&selector_root).unwrap();
                assert_eq!(selector.verify_selected(&reopened).unwrap(), selected);
                assert_eq!(reopened.held_projection().unwrap(), projection);
            }
        }
    }
}
