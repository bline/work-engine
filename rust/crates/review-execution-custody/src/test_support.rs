//! Controlled fixture custody for J1 domain composition only. The fixed class,
//! owner, verifier and profile cannot be selected by a fixture caller.

use crate::contract::{record_digest, require_sha, require_text, sha256};
use crate::{
    CheckedExecutionResult, CheckedObservation, EvidenceClass, EvidenceReadError,
    ExecutionEvidenceOwner, ExecutionEvidenceRef, ExecutionProvenance, ROOT_PAYLOAD_MAX_BYTES,
};
use claim_evidence::codec::{JsValue, canonical_json, digest};
use claim_evidence::{CustodyEvidence, ProductionPathAdmissionBinding};

pub const FIXTURE_OWNER: &str = "review-execution-evidence";
pub const FIXTURE_PROFILE: &str = "controlled-fixture-v1";
pub const FIXTURE_VERIFIER: &str = "review-execution-evidence-fixture-v1";

#[derive(Clone, Debug)]
pub struct FixtureRecord {
    pub reference: ExecutionEvidenceRef,
    pub raw_result: Vec<u8>,
    pub provenance: ExecutionProvenance,
    pub observation: CheckedObservation,
    pub artifact_references: Vec<String>,
}

impl FixtureRecord {
    pub fn controlled(
        root_id: &str,
        record_id: &str,
        revision: &str,
        raw_result: Vec<u8>,
        provenance: ExecutionProvenance,
        observation: CheckedObservation,
        artifact_references: Vec<String>,
    ) -> Result<Self, EvidenceReadError> {
        let mut record = Self {
            reference: ExecutionEvidenceRef {
                schema_version: 1,
                owner: FIXTURE_OWNER.into(),
                root_id: root_id.into(),
                profile: FIXTURE_PROFILE.into(),
                attempt_id: provenance.binding.attempt_id.clone(),
                record_id: record_id.into(),
                revision: revision.into(),
                sha256: String::new(),
            },
            raw_result,
            provenance,
            observation,
            artifact_references,
        };
        record.reference.sha256 = record.compute_digest()?;
        record.validate()?;
        Ok(record)
    }

    fn compute_digest(&self) -> Result<String, EvidenceReadError> {
        if self.raw_result.len() > crate::RESULT_MAX_BYTES {
            return Err(EvidenceReadError::Capacity("result exceeds limit"));
        }
        let obs_digest = match &self.observation {
            CheckedObservation::Present(value) => {
                if canonical_json(value)
                    .map_err(|_| EvidenceReadError::Conflicting("invalid fixture observation"))?
                    .len()
                    > crate::OBSERVATION_MAX_BYTES
                {
                    return Err(EvidenceReadError::Capacity("observation exceeds limit"));
                }
                digest(value)
                    .map_err(|_| EvidenceReadError::Conflicting("invalid fixture observation"))?
            }
            CheckedObservation::Absent { reason } => {
                require_text(reason)?;
                sha256(reason.as_bytes())
            }
        };
        let artifact_fields: Vec<&[u8]> = self
            .artifact_references
            .iter()
            .map(String::as_bytes)
            .collect();
        let artifact_digest = record_digest(&artifact_fields);
        let b = &self.provenance.binding;
        Ok(record_digest(&[
            &self.reference.schema_version.to_be_bytes(),
            self.reference.owner.as_bytes(),
            self.reference.root_id.as_bytes(),
            self.reference.profile.as_bytes(),
            self.reference.attempt_id.as_bytes(),
            self.reference.record_id.as_bytes(),
            self.reference.revision.as_bytes(),
            b.campaign_root_id.as_bytes(),
            b.obligation_id.as_bytes(),
            b.candidate_digest.as_bytes(),
            b.selection_digest.as_bytes(),
            b.profile_digest.as_bytes(),
            b.prepared_request_sha256.as_bytes(),
            b.review_episode_id.as_bytes(),
            b.attempt_id.as_bytes(),
            self.provenance.dispatch_operation_id.as_bytes(),
            self.provenance.dispatch_revision.as_bytes(),
            self.provenance
                .execution_session_id
                .as_deref()
                .unwrap_or("")
                .as_bytes(),
            sha256(&self.raw_result).as_bytes(),
            obs_digest.as_bytes(),
            artifact_digest.as_bytes(),
        ]))
    }

    fn validate(&self) -> Result<CheckedExecutionResult, EvidenceReadError> {
        self.reference.validate()?;
        if self.reference.owner != FIXTURE_OWNER || self.reference.profile != FIXTURE_PROFILE {
            return Err(EvidenceReadError::Unsupported("fixture profile is fixed"));
        }
        if self.reference.sha256 != self.compute_digest()? {
            return Err(EvidenceReadError::Conflicting(
                "fixture record digest mismatch",
            ));
        }
        let checked = CheckedExecutionResult::from_verified_parts(
            self.reference.clone(),
            &self.raw_result,
            self.provenance.clone(),
            self.observation.clone(),
            EvidenceClass::ControlledFixture,
        )?;
        match &self.observation {
            CheckedObservation::Present(value) => {
                let refs = artifact_refs(value)?;
                if refs != self.artifact_references {
                    return Err(EvidenceReadError::Conflicting(
                        "fixture artifact set mismatch",
                    ));
                }
                let execution = field(value, "execution")?;
                if text_field(execution, "attemptId")? != self.reference.attempt_id
                    || text_field(execution, "resultDigest")? != checked.claim_sha256()
                    || text_field(value, "obligationId")? != self.provenance.binding.obligation_id
                    || text_field(field(value, "subject")?, "reviewEpisodeId")?
                        != self.provenance.binding.review_episode_id
                    || text_field(field(value, "selection")?, "revision")?
                        != self.provenance.binding.selection_digest
                    || Some(text_field(field(value, "continuity")?, "sessionId")?)
                        != self.provenance.execution_session_id
                {
                    return Err(EvidenceReadError::Conflicting(
                        "fixture observation binding mismatch",
                    ));
                }
            }
            CheckedObservation::Absent { .. } => {
                if !self.artifact_references.is_empty() {
                    return Err(EvidenceReadError::Conflicting("absence has artifacts"));
                }
            }
        }
        Ok(checked)
    }
}

#[derive(Clone, Debug)]
pub struct ControlledFixtureOwner {
    records: Vec<FixtureRecord>,
}

impl ControlledFixtureOwner {
    pub fn new(records: Vec<FixtureRecord>) -> Result<Self, EvidenceReadError> {
        let mut total = 0usize;
        for (i, record) in records.iter().enumerate() {
            record.validate()?;
            total = total
                .checked_add(record.raw_result.len())
                .ok_or(EvidenceReadError::Capacity("fixture payload overflow"))?;
            if let CheckedObservation::Present(value) = &record.observation {
                total = total
                    .checked_add(
                        canonical_json(value)
                            .map_err(|_| {
                                EvidenceReadError::Conflicting("invalid fixture observation")
                            })?
                            .len(),
                    )
                    .ok_or(EvidenceReadError::Capacity("fixture payload overflow"))?;
            }
            if total > ROOT_PAYLOAD_MAX_BYTES {
                return Err(EvidenceReadError::Capacity("fixture root exceeds limit"));
            }
            if records[..i]
                .iter()
                .any(|prior| prior.reference == record.reference)
            {
                return Err(EvidenceReadError::Conflicting(
                    "duplicate exact fixture record",
                ));
            }
        }
        Ok(Self { records })
    }

    fn find(&self, reference: &ExecutionEvidenceRef) -> Result<&FixtureRecord, EvidenceReadError> {
        reference.validate()?;
        if reference.owner != FIXTURE_OWNER || reference.profile != FIXTURE_PROFILE {
            return Err(EvidenceReadError::Unsupported(
                "fixture cannot serve this profile",
            ));
        }
        if let Some(record) = self
            .records
            .iter()
            .find(|record| &record.reference == reference)
        {
            return Ok(record);
        }
        if self.records.iter().any(|record| {
            record.reference.root_id == reference.root_id
                && record.reference.attempt_id == reference.attempt_id
                && record.reference.record_id == reference.record_id
                && record.reference.revision == reference.revision
        }) {
            return Err(EvidenceReadError::Conflicting(
                "record identity digest differs",
            ));
        }
        Err(EvidenceReadError::Absent)
    }
}

impl ExecutionEvidenceOwner for ControlledFixtureOwner {
    fn read_result(
        &self,
        reference: &ExecutionEvidenceRef,
    ) -> Result<CheckedExecutionResult, EvidenceReadError> {
        self.find(reference)?.validate()
    }

    fn read_custody(
        &self,
        reference: &ExecutionEvidenceRef,
        binding: &ProductionPathAdmissionBinding,
        observation: Option<&JsValue>,
    ) -> Result<CustodyEvidence, EvidenceReadError> {
        binding
            .validate()
            .map_err(|_| EvidenceReadError::Conflicting("invalid CE admission binding"))?;
        let record = self.find(reference)?;
        let checked = record.validate()?;
        if !checked.provenance().binding.matches_admission(binding)
            || checked.claim_sha256() != binding.native_result_sha256
        {
            return Err(EvidenceReadError::Conflicting(
                "execution/CE binding mismatch",
            ));
        }
        let (session_id, observation_sha256, artifact_references) = match checked.observation() {
            CheckedObservation::Present(stored) => {
                if observation != Some(stored) {
                    return Err(EvidenceReadError::Conflicting(
                        "observation differs from stored evidence",
                    ));
                }
                if binding.session_id.as_deref()
                    != checked.provenance().execution_session_id.as_deref()
                {
                    return Err(EvidenceReadError::Conflicting("CE session mismatch"));
                }
                (
                    binding.session_id.clone(),
                    Some(digest(stored).map_err(|_| {
                        EvidenceReadError::Conflicting("observation digest failed")
                    })?),
                    record.artifact_references.clone(),
                )
            }
            CheckedObservation::Absent { .. } => {
                if observation.is_some() || binding.session_id.is_some() {
                    return Err(EvidenceReadError::Conflicting(
                        "checked observation absence mismatch",
                    ));
                }
                (None, None, Vec::new())
            }
        };
        require_sha(&reference.sha256)?;
        Ok(CustodyEvidence {
            owner: FIXTURE_OWNER.into(),
            reference: reference.custody_reference(),
            revision: reference.revision.clone(),
            sha256: reference.sha256.clone(),
            verifier: FIXTURE_VERIFIER.into(),
            profile: FIXTURE_PROFILE.into(),
            attempt_id: reference.attempt_id.clone(),
            native_result_sha256: checked.claim_sha256().into(),
            session_id,
            observation_sha256,
            artifact_references,
        })
    }
}

fn field<'a>(value: &'a JsValue, key: &str) -> Result<&'a JsValue, EvidenceReadError> {
    value
        .get(key)
        .ok_or(EvidenceReadError::Conflicting("observation field missing"))
}

fn text_field(value: &JsValue, key: &str) -> Result<String, EvidenceReadError> {
    field(value, key)?
        .as_text()
        .map_err(|_| EvidenceReadError::Conflicting("observation text invalid"))?
        .to_string_checked()
        .map_err(|_| EvidenceReadError::Conflicting("observation text invalid"))
        .and_then(|s| {
            if s.is_empty() {
                Err(EvidenceReadError::Conflicting("observation text empty"))
            } else {
                require_text(&s)?;
                Ok(s)
            }
        })
}

fn artifact_refs(value: &JsValue) -> Result<Vec<String>, EvidenceReadError> {
    field(value, "artifacts")?
        .as_array()
        .map_err(|_| EvidenceReadError::Conflicting("artifact list invalid"))?
        .iter()
        .map(|artifact| Ok(text_field(artifact, "reference")?.to_owned()))
        .collect()
}
