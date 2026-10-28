//! Durable S6 source and verdict linkage. Published artifacts can predate their
//! row after a crash, but an unlinked artifact never establishes a stage.

use lifecycle_core::{ClockSample, Revision, TransitionId};
use lifecycle_semantic::{
    Candidate, Challenge, CheckedSemanticResult, MaterialInput, SemanticError, SemanticPolicy,
    SourceBasis, VerdictStatus,
};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use sha2::{Digest as _, Sha256};
use std::{
    fs::OpenOptions,
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

use crate::{SqliteLifecycleStore, StoreError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticCaptureRecord {
    pub capture_id: String,
    pub transition_id: String,
    pub basis_sha256: String,
    pub basis_artifact_sha256: String,
    pub semantic_revision: u64,
    pub eligibility: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticMaterialRejectionRecord {
    pub capture_id: String,
    pub transition_id: String,
    pub input_sha256: String,
    pub policy_id: String,
    pub schema_version: u32,
    pub semantic_revision: u64,
    pub error_code: String,
    pub eligibility: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CurrentSemanticMaterialBlock {
    PromptTooLarge,
    CapturedIssues(Vec<lifecycle_semantic::MaterialIssue>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticCompileRefusalRecord {
    pub transition_id: String,
    pub capture_id: String,
    pub invocation_id: String,
    pub input_sha256: String,
    pub output_sha256: String,
    pub reason: String,
}

#[cfg(test)]
mod current_material_block_tests {
    use super::*;
    use lifecycle_core::{
        BuildId, ContextGeneration, ProofRunId, SubjectId, WaitBudgetMs, WallTimeMs,
    };
    use lifecycle_semantic::{
        MaterialIssue, ObligationKind, RequiredObligation, SourceAvailability, SourceKind,
        SourceMaterial, SourceTrust, project_material, sha256_hex,
    };

    fn clock() -> ClockSample {
        ClockSample {
            wall: WallTimeMs::new(100),
            wait_budget: WaitBudgetMs::new(5),
        }
    }

    #[test]
    fn current_prompt_refusal_and_all_typed_captured_issues_survive_reopen_without_stale_carryover()
    {
        let root = tempfile::tempdir().unwrap();
        let subject = SubjectId::parse("subject-material").unwrap();
        let transition = TransitionId::parse("transition-material").unwrap();
        let mut store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        store
            .register_subject(
                subject.clone(),
                ContextGeneration::parse("context-material").unwrap(),
                BuildId::parse("build-material").unwrap(),
                ProofRunId::parse("proof-material").unwrap(),
                clock(),
            )
            .unwrap();
        store.connection.execute("INSERT INTO transitions(transition_id,subject_id,predecessor_context,stage,basis_revision,created_wall_ms) VALUES(?1,?2,'context-material','verifying',0,100)",
            params![transition.as_str(),subject.as_str()]).unwrap();
        store.connection.execute("UPDATE subjects SET owner_kind='transition',owner_ref=?1,semantic_revision=1 WHERE subject_id=?2",
            params![transition.as_str(),subject.as_str()]).unwrap();
        store
            .connection
            .execute(
                "UPDATE transitions SET semantic_required=1 WHERE transition_id=?1",
                [transition.as_str()],
            )
            .unwrap();
        let source = SourceMaterial {
            id: "result".into(),
            owner_namespace: "task".into(),
            owner_ref: "task".into(),
            revision: 1,
            dependency_key: "result".into(),
            current: true,
            dependency_required: true,
            kind: SourceKind::ExternalData,
            trust: SourceTrust::UntrustedData,
            availability: SourceAvailability::Unavailable,
            bytes: Vec::new(),
            sha256: sha256_hex(b"missing"),
        };
        let input = MaterialInput {
            subject: subject.clone(),
            transition: transition.clone(),
            capture_id: "capture-prompt".into(),
            sources: vec![source.clone()],
            obligations: vec![RequiredObligation {
                id: "obligation-result".into(),
                kind: ObligationKind::CompletedConsequence,
                verifier_scope: "result".into(),
                description: "preserve observed result".into(),
                sources: vec![source.reference()],
            }],
            policy: SemanticPolicy::retained_repository_analysis(),
        };
        store
            .record_semantic_material_rejection(
                &input,
                &SemanticError::PromptTooLarge,
                Revision::new(1),
                clock(),
            )
            .unwrap();
        assert_eq!(
            store
                .current_semantic_material_block(&subject, &transition, "capture-prompt", 1)
                .unwrap(),
            Some(CurrentSemanticMaterialBlock::PromptTooLarge)
        );
        let mut issue_input = input.clone();
        issue_input.capture_id = "capture-issues".into();
        let basis = project_material(issue_input.clone()).unwrap();
        assert!(
            basis
                .issues
                .iter()
                .any(|i| matches!(i, MaterialIssue::Unavailable { .. }))
        );
        assert!(
            basis
                .issues
                .iter()
                .any(|i| matches!(i, MaterialIssue::RequiredSourceUnavailable { .. }))
        );
        store
            .record_semantic_capture(&basis, &issue_input.policy, Revision::new(1), clock())
            .unwrap();
        drop(store);
        let store = SqliteLifecycleStore::open(root.path(), "trusted".into()).unwrap();
        assert_eq!(
            store
                .current_semantic_material_block(&subject, &transition, "capture-issues", 1)
                .unwrap(),
            Some(CurrentSemanticMaterialBlock::CapturedIssues(
                basis.issues.clone()
            ))
        );
        store
            .connection
            .execute(
                "UPDATE subjects SET semantic_revision=2 WHERE subject_id=?1",
                [subject.as_str()],
            )
            .unwrap();
        assert_eq!(
            store
                .current_semantic_material_block(&subject, &transition, "capture-issues", 1)
                .unwrap(),
            None
        );
        assert_eq!(
            store
                .current_semantic_material_block(&subject, &transition, "capture-prompt", 2)
                .unwrap(),
            None
        );
    }
}

struct SettledCompileRow {
    subject: String,
    eligibility: String,
    phase: String,
    attempt_state: String,
    result_sha: Option<String>,
    native_state: String,
    native_sha: Option<String>,
    stage: String,
    prompt_sha: String,
    artifact_sha: Option<String>,
}

fn compile_refusal_event_kind(error: &SemanticError) -> Option<&'static str> {
    match error {
        SemanticError::InvalidOutput => Some("semantic_compile_invalid_output"),
        SemanticError::InvalidReference => Some("semantic_compile_invalid_reference"),
        SemanticError::OutputTooLarge => Some("semantic_compile_output_too_large"),
        _ => None,
    }
}

struct DigestWriter(Sha256);
impl Write for DigestWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn material_digest(input: &MaterialInput) -> Result<String, StoreError> {
    let mut writer = DigestWriter(Sha256::new());
    serde_json::to_writer(&mut writer, input).map_err(|_| StoreError::Rejected)?;
    Ok(writer
        .0
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn material_error_code(error: &SemanticError) -> &'static str {
    match error {
        SemanticError::InvalidIdentifier => "invalid_identifier",
        SemanticError::DuplicateIdentifier => "duplicate_identifier",
        SemanticError::InvalidDigest => "invalid_digest",
        SemanticError::InvalidSource => "invalid_source",
        SemanticError::InvalidReference => "invalid_reference",
        SemanticError::WrongBinding => "wrong_binding",
        SemanticError::WrongPurpose => "wrong_purpose",
        SemanticError::OutputTooLarge => "output_too_large",
        SemanticError::InvalidOutput => "invalid_output",
        SemanticError::PromptTooLarge => "prompt_too_large",
        SemanticError::IneligibleMaterial => "ineligible_material",
        SemanticError::UnsupportedPolicy => "unsupported_policy",
    }
}

fn optional_capture_barrier(root: Option<&Path>, point: &str) -> Result<(), StoreError> {
    let Some(root) = root else { return Ok(()) };
    let arm = root.join(format!("{point}.arm"));
    if !arm.exists() {
        return Ok(());
    }
    let mut reached = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(format!("{point}.reached")))
        .map_err(|_| StoreError::Unavailable)?;
    reached
        .write_all(b"reached\n")
        .map_err(|_| StoreError::Unavailable)?;
    reached.sync_all().map_err(|_| StoreError::Unavailable)?;
    let release = root.join(format!("{point}.release"));
    let deadline = Instant::now() + Duration::from_secs(15);
    while !release.exists() {
        if Instant::now() >= deadline {
            return Err(StoreError::Unavailable);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticResultRecord {
    pub result_id: String,
    pub transition_id: String,
    pub capture_id: String,
    pub phase: String,
    pub verifier_invocation_id: String,
    pub candidate_artifact_sha256: String,
    pub findings_artifact_sha256: String,
    pub verdict: String,
    pub eligibility: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticCheckpointPackage {
    pub result_id: String,
    pub basis: SourceBasis,
    pub candidate: Candidate,
    pub challenge: Challenge,
    pub successor_prompt: String,
    pub checked: CheckedSemanticResult,
}

impl SqliteLifecycleStore {
    /// A completed native turn can be semantically unusable. Keep its native
    /// settlement intact and bind the refusal to the exact current attempt.
    #[allow(clippy::too_many_arguments)]
    pub fn record_semantic_compile_refusal(
        &mut self,
        transition: &TransitionId,
        capture_id: &str,
        invocation_id: &str,
        input_sha256: &str,
        output_sha256: &str,
        error: &SemanticError,
        clock: ClockSample,
    ) -> Result<SemanticCompileRefusalRecord, StoreError> {
        self.verify_identity()?;
        let reason = compile_refusal_event_kind(error).ok_or(StoreError::Rejected)?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let row: Option<SettledCompileRow> = tx
            .query_row(
                "SELECT c.subject_id,c.eligibility,a.phase,a.state,a.result_sha256,n.state,n.final_sha256,tr.stage,a.input_sha256,a.result_artifact_sha256 \
                 FROM semantic_attempts a JOIN semantic_captures c ON c.capture_id=a.capture_id \
                 JOIN native_turns n ON n.invocation_id=a.invocation_id \
                 JOIN transitions tr ON tr.transition_id=c.transition_id AND tr.subject_id=c.subject_id \
                 WHERE a.invocation_id=?1 AND a.capture_id=?2 AND c.transition_id=?3",
                params![invocation_id, capture_id, transition.as_str()],
                |r| Ok(SettledCompileRow {
                    subject: r.get(0)?,
                    eligibility: r.get(1)?,
                    phase: r.get(2)?,
                    attempt_state: r.get(3)?,
                    result_sha: r.get(4)?,
                    native_state: r.get(5)?,
                    native_sha: r.get(6)?,
                    stage: r.get(7)?,
                    prompt_sha: r.get(8)?,
                    artifact_sha: r.get(9)?,
                }),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        let Some(row) = row else {
            return Err(StoreError::Rejected);
        };
        if row.eligibility != "current"
            || row.phase != "compile"
            || row.attempt_state != "completed_settled"
            || row.native_state != "completed_settled"
            || row.stage != "verifying"
            || row.result_sha.as_deref() != Some(output_sha256)
            || row.native_sha.as_deref() != Some(output_sha256)
            || row.prompt_sha != input_sha256
            || row.artifact_sha.is_some()
        {
            return Err(StoreError::Rejected);
        }
        let prior: Option<String> = tx.query_row(
            "SELECT event_kind FROM journal WHERE subject_id=?1 AND event_ref=?2 AND event_kind IN ('semantic_compile_invalid_output','semantic_compile_invalid_reference','semantic_compile_output_too_large') ORDER BY sequence DESC LIMIT 1",
            params![&row.subject, invocation_id], |r| r.get(0),
        ).optional().map_err(|_| StoreError::Unavailable)?;
        if let Some(prior) = prior {
            if prior != reason {
                return Err(StoreError::ClaimConflict);
            }
        } else {
            tx.execute(
                "INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) VALUES (?1,?2,?3,?4)",
                params![&row.subject, reason, invocation_id, clock.wall.get()],
            )
            .map_err(|_| StoreError::Unavailable)?;
        }
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(SemanticCompileRefusalRecord {
            transition_id: transition.as_str().into(),
            capture_id: capture_id.into(),
            invocation_id: invocation_id.into(),
            input_sha256: input_sha256.into(),
            output_sha256: output_sha256.into(),
            reason: reason.into(),
        })
    }

    pub fn semantic_compile_refusal(
        &self,
        transition: &TransitionId,
        capture_id: &str,
        invocation_id: &str,
    ) -> Result<Option<SemanticCompileRefusalRecord>, StoreError> {
        self.verify_identity()?;
        self.connection.query_row(
            "SELECT c.transition_id,a.capture_id,a.invocation_id,a.input_sha256,a.result_sha256,j.event_kind \
             FROM journal j JOIN semantic_attempts a ON a.invocation_id=j.event_ref \
             JOIN semantic_captures c ON c.capture_id=a.capture_id \
             JOIN native_turns n ON n.invocation_id=a.invocation_id \
             WHERE c.transition_id=?1 AND a.capture_id=?2 AND a.invocation_id=?3 \
             AND c.eligibility='current' AND a.phase='compile' AND a.state='completed_settled' \
             AND n.state='completed_settled' AND n.final_sha256=a.result_sha256 \
             AND a.result_artifact_sha256 IS NULL AND j.subject_id=c.subject_id \
             AND j.event_kind IN ('semantic_compile_invalid_output','semantic_compile_invalid_reference','semantic_compile_output_too_large') \
             ORDER BY j.sequence DESC LIMIT 1",
            params![transition.as_str(),capture_id,invocation_id],
            |r| Ok(SemanticCompileRefusalRecord {
                transition_id:r.get(0)?,capture_id:r.get(1)?,invocation_id:r.get(2)?,
                input_sha256:r.get(3)?,output_sha256:r.get(4)?,reason:r.get(5)?,
            }),
        ).optional().map_err(|_|StoreError::Unavailable)
    }

    pub fn record_semantic_material_rejection(
        &mut self,
        input: &MaterialInput,
        error: &SemanticError,
        expected_semantic_revision: Revision,
        clock: ClockSample,
    ) -> Result<SemanticMaterialRejectionRecord, StoreError> {
        self.verify_identity()?;
        if !crate::admin::admission_open(&self.connection)?
            || input.capture_id.is_empty()
            || input.capture_id.len() > 128
            || !input
                .capture_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:/".contains(&byte))
        {
            return Err(StoreError::Rejected);
        }
        let record = SemanticMaterialRejectionRecord {
            capture_id: input.capture_id.clone(),
            transition_id: input.transition.as_str().into(),
            input_sha256: material_digest(input)?,
            policy_id: input.policy.id.clone(),
            schema_version: input.policy.schema_version,
            semantic_revision: expected_semantic_revision.get(),
            error_code: material_error_code(error).into(),
            eligibility: "unresolved".into(),
        };
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let prior:Option<(String,String,String,i64,i64,String,String)>=tx.query_row(
            "SELECT transition_id,input_sha256,policy_id,schema_version,semantic_revision,error_code,eligibility FROM semantic_material_rejections WHERE capture_id=?1",
            [&record.capture_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?)),
        ).optional().map_err(|_|StoreError::Unavailable)?;
        if let Some((transition, digest, policy, schema, revision, code, eligibility)) = prior {
            if transition != record.transition_id
                || digest != record.input_sha256
                || policy != record.policy_id
                || schema != i64::from(record.schema_version)
                || revision < 0
                || revision as u64 != record.semantic_revision
                || code != record.error_code
                || eligibility != record.eligibility
            {
                return Err(StoreError::ClaimConflict);
            }
            return Ok(record);
        }
        let eligible:bool=tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM transitions tr JOIN subjects sub ON sub.subject_id=tr.subject_id WHERE tr.transition_id=?1 AND tr.subject_id=?2 AND tr.stage='verifying' AND tr.semantic_required=1 AND sub.owner_kind='transition' AND sub.owner_ref=tr.transition_id AND sub.semantic_revision=?3)",
            params![&record.transition_id,input.subject.as_str(),i64::try_from(record.semantic_revision).map_err(|_|StoreError::Rejected)?],|r|r.get(0),
        ).map_err(|_|StoreError::Unavailable)?;
        let already_captured: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM semantic_captures WHERE capture_id=?1)",
                [&record.capture_id],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        if !eligible || already_captured {
            return Err(StoreError::ClaimConflict);
        }
        tx.execute("INSERT INTO semantic_material_rejections(capture_id,transition_id,subject_id,input_sha256,policy_id,schema_version,semantic_revision,error_code,eligibility,created_wall_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![&record.capture_id,&record.transition_id,input.subject.as_str(),&record.input_sha256,
                &record.policy_id,i64::from(record.schema_version),i64::try_from(record.semantic_revision).map_err(|_|StoreError::Rejected)?,
                &record.error_code,&record.eligibility,clock.wall.get()]).map_err(|_|StoreError::ClaimConflict)?;
        tx.execute("INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) VALUES (?1,'semantic_material_rejected',?2,?3)",
            params![input.subject.as_str(),&record.capture_id,clock.wall.get()]).map_err(|_|StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(record)
    }

    pub fn semantic_material_rejection(
        &self,
        capture_id: &str,
    ) -> Result<Option<SemanticMaterialRejectionRecord>, StoreError> {
        self.verify_identity()?;
        self.connection.query_row("SELECT capture_id,transition_id,input_sha256,policy_id,schema_version,semantic_revision,error_code,eligibility FROM semantic_material_rejections WHERE capture_id=?1",
            [capture_id],|r|Ok(SemanticMaterialRejectionRecord{capture_id:r.get(0)?,transition_id:r.get(1)?,
                input_sha256:r.get(2)?,policy_id:r.get(3)?,schema_version:r.get::<_,i64>(4)? as u32,
                semantic_revision:r.get::<_,i64>(5)? as u64,error_code:r.get(6)?,eligibility:r.get(7)?}))
            .optional().map_err(|_|StoreError::Unavailable)
    }

    pub fn current_semantic_material_block(
        &self,
        subject: &lifecycle_core::SubjectId,
        transition: &TransitionId,
        capture_id: &str,
        semantic_revision: u64,
    ) -> Result<Option<CurrentSemanticMaterialBlock>, StoreError> {
        self.verify_identity()?;
        let current: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM transitions tr JOIN subjects s ON s.subject_id=tr.subject_id
             WHERE tr.transition_id=?1 AND tr.subject_id=?2 AND tr.stage='verifying'
               AND s.owner_kind='transition' AND s.owner_ref=tr.transition_id
               AND s.semantic_revision=?3)",
            params![transition.as_str(),subject.as_str(),i64::try_from(semantic_revision).map_err(|_|StoreError::Rejected)?],
            |r|r.get(0),
        ).map_err(|_|StoreError::Unavailable)?;
        if !current {
            return Ok(None);
        }
        let rejection: Option<String> = self
            .connection
            .query_row(
                "SELECT error_code FROM semantic_material_rejections
             WHERE capture_id=?1 AND transition_id=?2 AND subject_id=?3
               AND semantic_revision=?4 AND eligibility='unresolved'",
                params![
                    capture_id,
                    transition.as_str(),
                    subject.as_str(),
                    i64::try_from(semantic_revision).map_err(|_| StoreError::Rejected)?
                ],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        if rejection.as_deref() == Some("prompt_too_large") {
            return Ok(Some(CurrentSemanticMaterialBlock::PromptTooLarge));
        }
        let row: Option<(String, String)> = self
            .connection
            .query_row(
                "SELECT basis_sha256,basis_artifact_sha256 FROM semantic_captures
             WHERE capture_id=?1 AND transition_id=?2 AND subject_id=?3
               AND source_revision=?4 AND eligibility='unresolved'",
                params![
                    capture_id,
                    transition.as_str(),
                    subject.as_str(),
                    i64::try_from(semantic_revision).map_err(|_| StoreError::Rejected)?
                ],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        let Some((basis_digest, artifact_digest)) = row else {
            return Ok(None);
        };
        let bytes = self
            .read_committed_artifact(&artifact_digest)?
            .ok_or(StoreError::Unavailable)?;
        let basis: SourceBasis =
            serde_json::from_slice(&bytes).map_err(|_| StoreError::Unavailable)?;
        if basis.subject != *subject
            || basis.transition != *transition
            || basis.capture_id != capture_id
            || basis.sha256 != basis_digest
            || basis.verify_digest().is_err()
            || basis.issues.is_empty()
        {
            return Err(StoreError::ObservationMismatch);
        }
        Ok(Some(CurrentSemanticMaterialBlock::CapturedIssues(
            basis.issues,
        )))
    }

    pub fn require_semantic_transition(
        &mut self,
        transition: &TransitionId,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let state:Option<(i64,String,String,String,bool)>=tx.query_row(
            "SELECT tr.semantic_required,tr.stage,sub.owner_kind,sub.owner_ref,tr.native_basis_source IS NOT NULL FROM transitions tr JOIN subjects sub ON sub.subject_id=tr.subject_id WHERE tr.transition_id=?1",
            [transition.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)),
        ).optional().map_err(|_|StoreError::Unavailable)?;
        let Some((required, stage, owner_kind, owner_ref, native_basis)) = state else {
            return Err(StoreError::Rejected);
        };
        if owner_kind != "transition" || owner_ref != transition.as_str() || !native_basis {
            return Err(StoreError::Rejected);
        }
        if required == 1 {
            return Ok(());
        }
        if required != 0 || !matches!(stage.as_str(), "quiescing" | "capturing" | "verifying") {
            return Err(StoreError::ClaimConflict);
        }
        tx.execute("UPDATE transitions SET semantic_required=1 WHERE transition_id=?1 AND semantic_required=0",
            [transition.as_str()]).map_err(|_|StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)
    }

    pub fn semantic_transition_required(
        &self,
        transition: &TransitionId,
    ) -> Result<bool, StoreError> {
        self.verify_identity()?;
        self.connection
            .query_row(
                "SELECT semantic_required FROM transitions WHERE transition_id=?1",
                [transition.as_str()],
                |r| r.get::<_, i64>(0),
            )
            .map(|value| value == 1)
            .map_err(|_| StoreError::Unavailable)
    }

    pub fn record_semantic_capture(
        &mut self,
        basis: &SourceBasis,
        policy: &SemanticPolicy,
        expected_semantic_revision: Revision,
        clock: ClockSample,
    ) -> Result<SemanticCaptureRecord, StoreError> {
        self.record_semantic_capture_with_barrier(
            basis,
            policy,
            expected_semantic_revision,
            clock,
            None,
        )
    }

    pub fn record_semantic_capture_with_barrier(
        &mut self,
        basis: &SourceBasis,
        policy: &SemanticPolicy,
        expected_semantic_revision: Revision,
        clock: ClockSample,
        barrier_dir: Option<&Path>,
    ) -> Result<SemanticCaptureRecord, StoreError> {
        self.verify_identity()?;
        if !crate::admin::admission_open(&self.connection)? || basis.validate(policy).is_err() {
            return Err(StoreError::Rejected);
        }
        let bytes = serde_json::to_vec(basis).map_err(|_| StoreError::Rejected)?;
        let staged = self.stage_artifact(basis.subject.clone(), &bytes)?;
        optional_capture_barrier(barrier_dir, "semantic_after_capture_staged_before_publish")?;
        let artifact = self.publish_artifact(staged, clock)?;
        optional_capture_barrier(barrier_dir, "semantic_after_capture_published_before_link")?;
        let record = SemanticCaptureRecord {
            capture_id: basis.capture_id.clone(),
            transition_id: basis.transition.as_str().into(),
            basis_sha256: basis.sha256.clone(),
            basis_artifact_sha256: artifact.digest_hex,
            semantic_revision: expected_semantic_revision.get(),
            eligibility: if basis.is_eligible() {
                "current"
            } else {
                "unresolved"
            }
            .into(),
        };
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let current:Option<(String,i64)>=tx.query_row(
            "SELECT sub.owner_kind,sub.semantic_revision FROM subjects sub JOIN transitions tr ON tr.subject_id=sub.subject_id AND tr.transition_id=sub.owner_ref WHERE sub.subject_id=?1 AND tr.transition_id=?2 AND tr.stage='verifying'",
            params![basis.subject.as_str(),basis.transition.as_str()],|r|Ok((r.get(0)?,r.get(1)?)),
        ).optional().map_err(|_|StoreError::Unavailable)?;
        if !matches!(current,Some((ref owner,revision)) if owner=="transition" && revision>=0 && revision as u64==expected_semantic_revision.get())
        {
            return Err(StoreError::ClaimConflict);
        }
        let prior:Option<(String,String,String,i64,String)>=tx.query_row(
            "SELECT transition_id,basis_sha256,basis_artifact_sha256,source_revision,eligibility FROM semantic_captures WHERE capture_id=?1",
            [&record.capture_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)),
        ).optional().map_err(|_|StoreError::Unavailable)?;
        if let Some((transition, sha, artifact, revision, eligibility)) = prior {
            if transition != record.transition_id
                || sha != record.basis_sha256
                || artifact != record.basis_artifact_sha256
                || revision < 0
                || revision as u64 != record.semantic_revision
                || eligibility != record.eligibility
            {
                return Err(StoreError::ClaimConflict);
            }
            return Ok(record);
        }
        let rejected: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM semantic_material_rejections WHERE capture_id=?1)",
                [&record.capture_id],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        if rejected {
            return Err(StoreError::ClaimConflict);
        }
        let dependencies =
            serde_json::to_vec(&basis.dependencies).map_err(|_| StoreError::Rejected)?;
        tx.execute("INSERT INTO semantic_captures(capture_id,transition_id,subject_id,schema_id,policy_id,basis_sha256,basis_artifact_sha256,dependencies_json,source_revision,created_wall_ms,eligibility) VALUES (?1,?2,?3,'source-basis.v1',?4,?5,?6,?7,?8,?9,?10)",
            params![&record.capture_id,&record.transition_id,basis.subject.as_str(),&basis.policy_id,
                &record.basis_sha256,&record.basis_artifact_sha256,&dependencies,
                i64::try_from(record.semantic_revision).map_err(|_|StoreError::Rejected)?,clock.wall.get(),&record.eligibility])
            .map_err(|_|StoreError::ClaimConflict)?;
        tx.execute("INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) VALUES (?1,'semantic_capture',?2,?3)",
            params![basis.subject.as_str(),&record.capture_id,clock.wall.get()])
            .map_err(|_|StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(record)
    }

    pub fn semantic_transition_basis(
        &self,
        transition: &TransitionId,
    ) -> Result<Option<SemanticCaptureRecord>, StoreError> {
        self.verify_identity()?;
        self.connection.query_row(
            "SELECT capture_id,transition_id,basis_sha256,basis_artifact_sha256,source_revision FROM semantic_captures WHERE transition_id=?1 AND eligibility='current' ORDER BY created_wall_ms DESC,rowid DESC LIMIT 1",
            [transition.as_str()],|r|Ok(SemanticCaptureRecord {capture_id:r.get(0)?,transition_id:r.get(1)?,
            basis_sha256:r.get(2)?,basis_artifact_sha256:r.get(3)?,semantic_revision:r.get::<_,i64>(4)? as u64,
                eligibility:"current".into()}),
        ).optional().map_err(|_|StoreError::Unavailable)
    }

    pub fn semantic_capture_record(
        &self,
        capture_id: &str,
    ) -> Result<Option<SemanticCaptureRecord>, StoreError> {
        self.verify_identity()?;
        self.connection.query_row(
            "SELECT capture_id,transition_id,basis_sha256,basis_artifact_sha256,source_revision,eligibility FROM semantic_captures WHERE capture_id=?1",
            [capture_id],|r|Ok(SemanticCaptureRecord {capture_id:r.get(0)?,transition_id:r.get(1)?,
                basis_sha256:r.get(2)?,basis_artifact_sha256:r.get(3)?,semantic_revision:r.get::<_,i64>(4)? as u64,
                eligibility:r.get(5)?}),
        ).optional().map_err(|_|StoreError::Unavailable)
    }

    /// An owner revision change retires this exact basis and its derived results
    /// without rewriting the historical bytes or verifier findings.
    pub fn mark_semantic_capture_stale(
        &mut self,
        capture_id: &str,
        basis_sha256: &str,
        clock: ClockSample,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let subject:Option<(String,String,String)>=tx.query_row(
            "SELECT subject_id,basis_sha256,eligibility FROM semantic_captures WHERE capture_id=?1",
            [capture_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
        ).optional().map_err(|_|StoreError::Unavailable)?;
        let Some((subject, sha, status)) = subject else {
            return Err(StoreError::Rejected);
        };
        if sha != basis_sha256 || !matches!(status.as_str(), "current" | "stale") {
            return Err(StoreError::ClaimConflict);
        }
        if status == "current" {
            tx.execute("UPDATE semantic_captures SET eligibility='stale' WHERE capture_id=?1 AND eligibility='current'",
                [capture_id]).map_err(|_|StoreError::Unavailable)?;
            tx.execute("UPDATE semantic_results SET eligibility='stale' WHERE capture_id=?1 AND eligibility='current'",
                [capture_id]).map_err(|_|StoreError::Unavailable)?;
            tx.execute("INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) VALUES (?1,'semantic_capture_stale',?2,?3)",
                params![subject,capture_id,clock.wall.get()]).map_err(|_|StoreError::Unavailable)?;
        }
        tx.commit().map_err(|_| StoreError::Unavailable)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_semantic_result(
        &mut self,
        result_id: &str,
        capture_id: &str,
        compile_invocation_id: &str,
        candidate_artifact_sha256: &str,
        challenge_artifact_sha256: Option<&str>,
        response_artifact_sha256: Option<&str>,
        checked: &CheckedSemanticResult,
        clock: ClockSample,
    ) -> Result<SemanticResultRecord, StoreError> {
        self.verify_identity()?;
        if !crate::admin::admission_open(&self.connection)? {
            return Err(StoreError::Rejected);
        }
        let bytes = serde_json::to_vec(checked).map_err(|_| StoreError::Rejected)?;
        let (subject,transition,basis_sha,policy): (String,String,String,String)=self.connection.query_row(
            "SELECT subject_id,transition_id,basis_sha256,policy_id FROM semantic_captures WHERE capture_id=?1 AND eligibility='current'",
            [capture_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
        ).map_err(|_|StoreError::Rejected)?;
        if checked.source_basis_sha256 != basis_sha
            || checked.policy_id != policy
            || checked.candidate_sha256 != candidate_artifact_sha256
            || checked.successor_response_sha256.as_deref() != response_artifact_sha256
        {
            return Err(StoreError::Rejected);
        }
        if let Some(challenge_artifact_sha256) = challenge_artifact_sha256 {
            let challenge_bytes = self
                .read_committed_artifact(challenge_artifact_sha256)?
                .ok_or(StoreError::Rejected)?;
            let challenge: Challenge =
                serde_json::from_slice(&challenge_bytes).map_err(|_| StoreError::Rejected)?;
            if checked.challenge_sha256.as_deref() != Some(challenge.sha256.as_str()) {
                return Err(StoreError::Rejected);
            }
        } else if checked.challenge_sha256.is_some() {
            return Err(StoreError::Rejected);
        }
        let staged = self.stage_artifact(
            lifecycle_core::SubjectId::parse(subject.clone()).map_err(|_| StoreError::Rejected)?,
            &bytes,
        )?;
        let findings = self.publish_artifact(staged, clock)?;
        let verdict = match checked.status {
            VerdictStatus::Accepted => "accepted",
            VerdictStatus::Rejected => "rejected",
            VerdictStatus::Unresolved => "unresolved",
        };
        let phase = match checked.phase {
            lifecycle_semantic::SemanticPhase::Compile => return Err(StoreError::Rejected),
            lifecycle_semantic::SemanticPhase::CandidateVerification => "candidate_verify",
            lifecycle_semantic::SemanticPhase::SuccessorAdequacy => "successor_adequacy",
        };
        let record = SemanticResultRecord {
            result_id: result_id.into(),
            transition_id: transition,
            capture_id: capture_id.into(),
            phase: phase.into(),
            verifier_invocation_id: checked.verifier_invocation_id.clone(),
            candidate_artifact_sha256: candidate_artifact_sha256.into(),
            findings_artifact_sha256: findings.digest_hex,
            verdict: verdict.into(),
            eligibility: "current".into(),
        };
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let linked:Option<(String,String,String,String)>=tx.query_row(
            "SELECT ca.result_sha256,va.result_sha256,ca.capture_id,va.capture_id FROM semantic_attempts ca JOIN semantic_attempts va ON va.invocation_id=?2 WHERE ca.invocation_id=?1 AND ca.phase='compile' AND ca.state='completed_settled' AND va.state='completed_settled' AND va.phase=?3",
            params![compile_invocation_id,&checked.verifier_invocation_id,phase],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
        ).optional().map_err(|_|StoreError::Unavailable)?;
        if !matches!(linked,Some((ref candidate,ref verifier,ref c,ref v)) if candidate==candidate_artifact_sha256 && verifier==&checked.verifier_result_sha256 && c==capture_id && v==capture_id)
        {
            return Err(StoreError::ClaimConflict);
        }
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM artifacts WHERE digest_hex=?1)",
                [candidate_artifact_sha256],
                |r| r.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        if !exists {
            return Err(StoreError::Rejected);
        }
        let prior:Option<(String,String,String,String)>=tx.query_row(
            "SELECT capture_id,verifier_invocation_id,candidate_artifact_sha256,findings_artifact_sha256 FROM semantic_results WHERE result_id=?1",
            [result_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
        ).optional().map_err(|_|StoreError::Unavailable)?;
        if let Some((c, v, a, f)) = prior {
            if c != capture_id
                || v != record.verifier_invocation_id
                || a != candidate_artifact_sha256
                || f != record.findings_artifact_sha256
            {
                return Err(StoreError::ClaimConflict);
            }
            return Ok(record);
        }
        tx.execute("UPDATE semantic_attempts SET result_artifact_sha256=?2 WHERE invocation_id=?1 AND result_sha256=?2",
            params![compile_invocation_id,candidate_artifact_sha256]).map_err(|_|StoreError::Unavailable)?;
        tx.execute("INSERT INTO semantic_results(result_id,transition_id,capture_id,verifier_invocation_id,phase,candidate_artifact_sha256,challenge_artifact_sha256,response_artifact_sha256,findings_artifact_sha256,verdict,eligibility,basis_sha256,policy_id,created_wall_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'current',?11,?12,?13)",
            params![result_id,&record.transition_id,capture_id,&record.verifier_invocation_id,phase,candidate_artifact_sha256,
                challenge_artifact_sha256,response_artifact_sha256,&record.findings_artifact_sha256,verdict,&basis_sha,&policy,clock.wall.get()])
            .map_err(|_|StoreError::ClaimConflict)?;
        tx.execute("INSERT INTO journal(subject_id,event_kind,event_ref,wall_ms) VALUES (?1,'semantic_result',?2,?3)",
            params![&subject,result_id,clock.wall.get()]).map_err(|_|StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(record)
    }

    pub fn link_semantic_attempt_artifact(
        &mut self,
        invocation_id: &str,
        artifact_sha256: &str,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let matching:bool=tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM semantic_attempts a JOIN native_turns n ON n.invocation_id=a.invocation_id JOIN artifacts b ON b.digest_hex=?2 WHERE a.invocation_id=?1 AND a.state='completed_settled' AND n.state='completed_settled' AND a.result_sha256=?2 AND n.final_sha256=?2 AND (a.result_artifact_sha256 IS NULL OR a.result_artifact_sha256=?2))",
            params![invocation_id,artifact_sha256],|r|r.get(0),
        ).map_err(|_|StoreError::Unavailable)?;
        if !matching {
            return Err(StoreError::ClaimConflict);
        }
        tx.execute("UPDATE semantic_attempts SET result_artifact_sha256=?2 WHERE invocation_id=?1 AND result_artifact_sha256 IS NULL",
            params![invocation_id,artifact_sha256]).map_err(|_|StoreError::Unavailable)?;
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(())
    }

    pub fn publish_semantic_checkpoint(
        &mut self,
        result_id: &str,
        candidate: &Candidate,
        challenge: &Challenge,
        successor_prompt: &str,
        clock: ClockSample,
    ) -> Result<(), StoreError> {
        self.verify_identity()?;
        if successor_prompt.is_empty()
            || successor_prompt.len() > 65_536
            || candidate.sha256 != lifecycle_semantic::sha256_hex(&candidate.raw)
            || challenge.candidate_sha256 != candidate.sha256
            || challenge.source_basis_sha256 != candidate.source_basis_sha256
        {
            return Err(StoreError::Rejected);
        }
        let (subject,candidate_digest,basis_digest,status):(String,String,String,String)=self.connection.query_row(
            "SELECT c.subject_id,r.candidate_artifact_sha256,r.basis_sha256,r.verdict FROM semantic_results r JOIN semantic_captures c ON c.capture_id=r.capture_id WHERE r.result_id=?1 AND r.eligibility='current' AND c.eligibility='current'",
            [result_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
        ).map_err(|_|StoreError::Rejected)?;
        if status != "accepted"
            || candidate_digest != candidate.sha256
            || basis_digest != candidate.source_basis_sha256
        {
            return Err(StoreError::Rejected);
        }
        let subject =
            lifecycle_core::SubjectId::parse(subject).map_err(|_| StoreError::Unavailable)?;
        let candidate_record = self.publish_artifact(
            self.stage_artifact(
                subject.clone(),
                &serde_json::to_vec(candidate).map_err(|_| StoreError::Rejected)?,
            )?,
            clock,
        )?;
        let challenge_record = self.publish_artifact(
            self.stage_artifact(
                subject.clone(),
                &serde_json::to_vec(challenge).map_err(|_| StoreError::Rejected)?,
            )?,
            clock,
        )?;
        let prompt_record = self.publish_artifact(
            self.stage_artifact(subject, successor_prompt.as_bytes())?,
            clock,
        )?;
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| StoreError::Unavailable)?;
        let changed=tx.execute("UPDATE semantic_results SET candidate_record_artifact_sha256=?2,challenge_artifact_sha256=?3,successor_prompt_artifact_sha256=?4 WHERE result_id=?1 AND verdict='accepted' AND eligibility='current' AND candidate_record_artifact_sha256 IS NULL AND challenge_artifact_sha256 IS NULL AND successor_prompt_artifact_sha256 IS NULL",
            params![result_id,&candidate_record.digest_hex,&challenge_record.digest_hex,&prompt_record.digest_hex])
            .map_err(|_|StoreError::Unavailable)?;
        if changed != 1 {
            let prior:Option<(String,String,String)>=tx.query_row("SELECT candidate_record_artifact_sha256,challenge_artifact_sha256,successor_prompt_artifact_sha256 FROM semantic_results WHERE result_id=?1",
                [result_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|_|StoreError::Unavailable)?;
            if prior
                != Some((
                    candidate_record.digest_hex,
                    challenge_record.digest_hex,
                    prompt_record.digest_hex,
                ))
            {
                return Err(StoreError::ClaimConflict);
            }
        }
        tx.commit().map_err(|_| StoreError::Unavailable)?;
        Ok(())
    }

    pub fn semantic_checkpoint(
        &self,
        transition: &TransitionId,
    ) -> Result<Option<SemanticCheckpointPackage>, StoreError> {
        self.verify_identity()?;
        let row:Option<(String,String,String,String,String,String)>=self.connection.query_row(
            "SELECT r.result_id,c.basis_artifact_sha256,r.candidate_record_artifact_sha256,r.challenge_artifact_sha256,r.successor_prompt_artifact_sha256,r.findings_artifact_sha256 FROM semantic_results r JOIN semantic_captures c ON c.capture_id=r.capture_id WHERE r.transition_id=?1 AND r.verdict='accepted' AND r.eligibility='current' AND c.eligibility='current' AND r.candidate_record_artifact_sha256 IS NOT NULL AND r.challenge_artifact_sha256 IS NOT NULL AND r.successor_prompt_artifact_sha256 IS NOT NULL ORDER BY r.created_wall_ms DESC,r.rowid DESC LIMIT 1",
            [transition.as_str()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)),
        ).optional().map_err(|_|StoreError::Unavailable)?;
        let Some((result_id, basis, candidate, challenge, prompt, findings)) = row else {
            return Ok(None);
        };
        let read = |sha: &str| {
            self.read_committed_artifact(sha)?
                .ok_or(StoreError::Unavailable)
        };
        let basis: SourceBasis =
            serde_json::from_slice(&read(&basis)?).map_err(|_| StoreError::Unavailable)?;
        let candidate: Candidate =
            serde_json::from_slice(&read(&candidate)?).map_err(|_| StoreError::Unavailable)?;
        let challenge: Challenge =
            serde_json::from_slice(&read(&challenge)?).map_err(|_| StoreError::Unavailable)?;
        let successor_prompt =
            String::from_utf8(read(&prompt)?).map_err(|_| StoreError::Unavailable)?;
        let checked: CheckedSemanticResult =
            serde_json::from_slice(&read(&findings)?).map_err(|_| StoreError::Unavailable)?;
        if basis.transition != *transition
            || candidate.source_basis_sha256 != basis.sha256
            || challenge.candidate_sha256 != candidate.sha256
            || checked.status != VerdictStatus::Accepted
            || checked.candidate_sha256 != candidate.sha256
            || checked.source_basis_sha256 != basis.sha256
        {
            return Err(StoreError::Unavailable);
        }
        Ok(Some(SemanticCheckpointPackage {
            result_id,
            basis,
            candidate,
            challenge,
            successor_prompt,
            checked,
        }))
    }

    pub fn semantic_successor_verdict(
        &self,
        transition: &TransitionId,
    ) -> Result<Option<(String, String)>, StoreError> {
        self.verify_identity()?;
        self.connection.query_row(
            "SELECT result_id,verdict FROM semantic_results WHERE transition_id=?1 AND phase='successor_adequacy' ORDER BY created_wall_ms DESC,rowid DESC LIMIT 1",
            [transition.as_str()],|r|Ok((r.get(0)?,r.get(1)?)),
        ).optional().map_err(|_|StoreError::Unavailable)
    }
}
