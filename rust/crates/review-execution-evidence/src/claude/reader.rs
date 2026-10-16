use std::path::{Path, PathBuf};

use claim_evidence::codec::{JsValue, parse_json};
use claim_evidence::{CustodyEvidence, ProductionPathAdmissionBinding};

use crate::contract::{require_text, sha256};
use crate::store::{self, ReaderPin};
use crate::{
    CheckedExecutionResult, CheckedObservation, EvidenceClass, EvidenceReadError,
    EvidenceReaderProfile, ExecutionEvidenceOwner, ExecutionEvidenceRef, OBSERVATION_MAX_BYTES,
    REQUEST_MAX_BYTES, RESULT_MAX_BYTES, STREAM_MAX_BYTES,
};

use super::qualified::{
    Prepared, QualifiedReaderSelection, command_digest, parse_stored_subject,
    validate_stored_prompt,
};
use super::session::{
    RetainedReviewSession, SessionLocator, SessionManifest, SessionReady, manifest_digest,
    verify_chain, verify_live, verify_snapshot,
};
use super::transport::validate_response;
use super::writer::{CompleteCapture, QualifiedTerminal};

pub struct QualifiedReader {
    root: PathBuf,
    selection: QualifiedReaderSelection,
    anchors: store::AnchorIds,
}

impl QualifiedReader {
    pub fn open(
        root: &Path,
        selection: QualifiedReaderSelection,
    ) -> Result<Self, EvidenceReadError> {
        let expected_profile = if selection.profile.installed() {
            store::CLAUDE_PROFILE
        } else {
            store::CLAUDE_TEST_PROFILE
        };
        if selection.pin.profile != expected_profile {
            return Err(EvidenceReadError::Unsupported(
                "qualified reader profile unavailable",
            ));
        }
        selection
            .profile
            .verify()
            .map_err(|_| EvidenceReadError::Conflicting("qualified reader executable drift"))?;
        let (root, _, _) = store::open_reader(root, &selection.pin)?;
        let anchors = store::anchor_ids(&root)?;
        Ok(Self {
            root,
            selection,
            anchors,
        })
    }

    pub fn pin(&self) -> &ReaderPin {
        &self.selection.pin
    }

    pub fn retained_session(
        &self,
        reference: &ExecutionEvidenceRef,
    ) -> Result<RetainedReviewSession, EvidenceReadError> {
        let (_, _, _, ready, manifest) = self.read_checked(reference)?;
        let locator = SessionLocator {
            schema_version: 1,
            root_id: reference.root_id.clone(),
            profile: reference.profile.clone(),
            attempt_id: reference.attempt_id.clone(),
            session_id: self.selection.session_id.clone(),
            manifest_sha256: sha256(&serde_json::to_vec(&manifest).map_err(|_| {
                EvidenceReadError::Conflicting("session manifest serialization failed")
            })?),
        };
        RetainedReviewSession::checked(
            locator,
            self.root.clone(),
            &ready,
            vec![manifest],
            self.anchors,
        )
    }

    /// Reopen a later custody manifest linked to this exact initial attempt.
    /// This reads session state only; it cannot issue a provider entry.
    pub fn retained_session_after(
        &self,
        reference: &ExecutionEvidenceRef,
        successor_attempt: &str,
    ) -> Result<RetainedReviewSession, EvidenceReadError> {
        if successor_attempt == reference.attempt_id {
            return Err(EvidenceReadError::Unsupported(
                "successor attempt must differ",
            ));
        }
        let (ready, chain) = self.checked_chain(reference, successor_attempt)?;
        let latest = chain.last().unwrap();
        let locator = SessionLocator {
            schema_version: 1,
            root_id: reference.root_id.clone(),
            profile: reference.profile.clone(),
            attempt_id: latest.attempt_id.clone(),
            session_id: latest.session_id.clone(),
            manifest_sha256: manifest_digest(latest)?,
        };
        RetainedReviewSession::checked(locator, self.root.clone(), &ready, chain, self.anchors)
    }

    pub(crate) fn checked_chain(
        &self,
        reference: &ExecutionEvidenceRef,
        successor_attempt: &str,
    ) -> Result<(SessionReady, Vec<SessionManifest>), EvidenceReadError> {
        require_text(successor_attempt)?;
        let (_, _, _, ready, initial) = self.read_checked(reference)?;
        if successor_attempt == initial.attempt_id {
            return Ok((ready, vec![initial]));
        }
        let (_, _, conn) = store::open_reader(&self.root, &self.selection.pin)?;
        let mut backwards = Vec::new();
        let mut current = successor_attempt.to_owned();
        for _ in 0..63 {
            if current == initial.attempt_id {
                break;
            }
            let events = store::read_events(&conn, &current)?;
            if events.len() != 1 || events[0].stage != "session_evolution" {
                return Err(EvidenceReadError::Unresolved(
                    "successor session record absent",
                ));
            }
            let manifest: SessionManifest =
                serde_json::from_slice(&events[0].payload).map_err(|_| {
                    EvidenceReadError::Conflicting("successor session manifest invalid")
                })?;
            if manifest.attempt_id != current || manifest.session_id != initial.session_id {
                return Err(EvidenceReadError::Conflicting(
                    "successor session identity changed",
                ));
            }
            current =
                manifest
                    .predecessor_attempt_id
                    .clone()
                    .ok_or(EvidenceReadError::Conflicting(
                        "successor predecessor absent",
                    ))?;
            backwards.push(manifest);
        }
        if current != initial.attempt_id {
            return Err(EvidenceReadError::Capacity(
                "successor session chain too deep",
            ));
        }
        backwards.reverse();
        let mut chain = vec![initial];
        chain.extend(backwards);
        verify_chain(&self.root, &chain)?;
        if store::anchor_ids(&self.root)? != self.anchors {
            return Err(EvidenceReadError::Conflicting(
                "Claude reader anchors changed during successor read",
            ));
        }
        Ok((ready, chain))
    }

    fn read_checked(
        &self,
        reference: &ExecutionEvidenceRef,
    ) -> Result<
        (
            CheckedExecutionResult,
            QualifiedTerminal,
            CompleteCapture,
            SessionReady,
            SessionManifest,
        ),
        EvidenceReadError,
    > {
        reference.validate()?;
        let class = if self.selection.profile.installed() {
            EvidenceClass::QualifiedProvider
        } else {
            EvidenceClass::ControlledFixture
        };
        if self.selection.profile.installed() {
            EvidenceReaderProfile::QualifiedProvider.require_class(class)?;
        } else {
            EvidenceReaderProfile::ControlledFixture.require_class(class)?;
        }
        if reference.owner != store::OWNER
            || reference.profile != self.selection.pin.profile
            || reference.root_id != self.selection.pin.root_id
            || reference.attempt_id != self.selection.attempt_id
            || reference.record_id != "terminal"
            || reference.revision != "1"
        {
            return Err(EvidenceReadError::Unsupported(
                "reference outside selected offline Claude root",
            ));
        }
        let (_, _, conn) = store::open_reader(&self.root, &self.selection.pin)?;
        if store::anchor_ids(&self.root)? != self.anchors {
            return Err(EvidenceReadError::Conflicting(
                "Claude reader root/database/fence replaced",
            ));
        }
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM artifacts WHERE attempt_id=?1",
                [&reference.attempt_id],
                |row| row.get(0),
            )
            .map_err(|_| EvidenceReadError::Unresolved("Claude artifact inventory unavailable"))?;
        let guarded = self.selection.profile.guard().is_some();
        if count != if guarded { 7 } else { 6 } {
            return Err(EvidenceReadError::Conflicting(
                "Claude artifact inventory invalid",
            ));
        }
        let events = store::read_events(&conn, &reference.attempt_id)?;
        let stages: Vec<_> = events.iter().map(|item| item.stage.as_str()).collect();
        if stages
            != [
                "prepared",
                "session_ready",
                "launch_intent",
                "spawn_outcome",
                "capture_complete",
                "terminal",
            ]
        {
            return Err(EvidenceReadError::Unresolved(
                "Claude terminal stage chain incomplete",
            ));
        }
        if events[5].sha256 != reference.sha256 {
            return Err(EvidenceReadError::Conflicting(
                "Claude reference digest mismatch",
            ));
        }
        let prepared: Prepared = serde_json::from_slice(&events[0].payload)
            .map_err(|_| EvidenceReadError::Conflicting("Claude prepared record invalid"))?;
        let ready: SessionReady = serde_json::from_slice(&events[1].payload)
            .map_err(|_| EvidenceReadError::Conflicting("Claude session identity invalid"))?;
        let capture: CompleteCapture = serde_json::from_slice(&events[4].payload)
            .map_err(|_| EvidenceReadError::Conflicting("Claude capture record invalid"))?;
        let terminal: QualifiedTerminal = serde_json::from_slice(&events[5].payload)
            .map_err(|_| EvidenceReadError::Conflicting("Claude terminal record invalid"))?;
        if prepared.schema_version != 1
            || capture.schema_version != 1
            || terminal.schema_version != 1
            || prepared.pin()? != self.selection.pin
            || prepared.requested_session_id != self.selection.session_id
            || prepared.profile_digest != self.selection.profile.profile_digest()
            || prepared.executable_sha256 != self.selection.profile.program().sha256()
            || prepared.catalog_sha256 != self.selection.profile.catalog_sha256()
            || prepared.requested_model != self.selection.profile.requested_model()
            || prepared.requested_effort != self.selection.profile.requested_effort()
            || prepared.command_sha256
                != command_digest(
                    &self.selection.profile,
                    &prepared.requested_session_id,
                    guarded,
                )
            || prepared.guard_sha256.as_deref()
                != self.selection.profile.guard().map(|guard| guard.sha256())
            || capture.guard_ready != guarded
            || capture.cbm_observation_sha256.is_some() != guarded
            || (self.selection.profile.installed()
                && prepared.provenance_class != "qualified_provider")
            || (!self.selection.profile.installed() && prepared.provenance_class != "offline_peer")
            || prepared.boundary_kind
                != if guarded {
                    "rp0-guarded-v1"
                } else {
                    "offline-direct-v1"
                }
            || events[3].payload != capture.pid.to_be_bytes()
            || capture.exit_code != 0
            || !capture.local_reaped
            || !capture.prompt_submitted
            || !capture.provider_reported_entry
            || capture.observed_session_id != prepared.requested_session_id
            || capture.session.session_id != prepared.requested_session_id
            || capture.session.attempt_id != prepared.attempt_id
            || capture.session.predecessor_sha256.is_some()
            || capture.session.predecessor_attempt_id.is_some()
            || terminal.capture_event_sha256 != events[4].sha256
        {
            return Err(EvidenceReadError::Conflicting(
                "Claude profile/capture binding mismatch",
            ));
        }
        prepared.provenance().validate()?;
        let prompt =
            store::read_artifact(&conn, &reference.attempt_id, "prompt", REQUEST_MAX_BYTES)?;
        validate_stored_prompt(&prompt, &prepared)?;
        let subject =
            store::read_artifact(&conn, &reference.attempt_id, "subject", REQUEST_MAX_BYTES)?;
        if sha256(&subject) != prepared.subject_sha256 {
            return Err(EvidenceReadError::Conflicting(
                "Claude stored subject digest mismatch",
            ));
        }
        let subject_value = parse_stored_subject(&subject)?;
        if subject_value.get("commit") != Some(&JsValue::text(&prepared.candidate_commit))
            || subject_value.get("tree") != Some(&JsValue::text(&prepared.candidate_tree))
            || subject_value.get("patchIdentity")
                != Some(&JsValue::text(&prepared.candidate_patch_identity))
        {
            return Err(EvidenceReadError::Conflicting(
                "Claude subject identity mismatch",
            ));
        }
        let response =
            store::read_artifact(&conn, &reference.attempt_id, "response", STREAM_MAX_BYTES)?;
        let stderr =
            store::read_artifact(&conn, &reference.attempt_id, "stderr", STREAM_MAX_BYTES)?;
        let result =
            store::read_artifact(&conn, &reference.attempt_id, "result", RESULT_MAX_BYTES)?;
        let observation_bytes = store::read_artifact(
            &conn,
            &reference.attempt_id,
            "observation",
            OBSERVATION_MAX_BYTES,
        )?;
        if let Some(expected) = &capture.cbm_observation_sha256 {
            let bytes = store::read_artifact(
                &conn,
                &reference.attempt_id,
                "cbm_observation",
                OBSERVATION_MAX_BYTES,
            )?;
            let value: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|_| EvidenceReadError::Conflicting("CBM observation invalid"))?;
            if sha256(&bytes) != *expected
                || value
                    .get("after_index_status")
                    .and_then(serde_json::Value::as_array)
                    .is_none_or(|items| items.len() < 2)
                || value
                    .get("mcp_request_count")
                    .and_then(serde_json::Value::as_u64)
                    .is_none_or(|count| count == 0)
            {
                return Err(EvidenceReadError::Conflicting(
                    "CBM observation readback invalid",
                ));
            }
        }
        if sha256(&response) != capture.response_sha256
            || sha256(&stderr) != capture.stderr_sha256
            || sha256(&result) != capture.result_sha256
            || terminal.result_sha256 != capture.result_sha256
            || sha256(&observation_bytes) != terminal.observation_sha256
        {
            return Err(EvidenceReadError::Conflicting(
                "Claude artifact digest mismatch",
            ));
        }
        let validated = validate_response(
            &response,
            &self.selection.profile,
            &prepared.requested_session_id,
            &subject_value,
        )?;
        if validated.result != result || validated.observed_model != capture.observed_model {
            return Err(EvidenceReadError::Conflicting(
                "Claude model/result readback mismatch",
            ));
        }
        verify_live(&self.root, &ready)?;
        verify_snapshot(&self.root, &capture.session)?;
        let manifest_bytes = serde_json::to_vec(&capture.session)
            .map_err(|_| EvidenceReadError::Conflicting("Claude session manifest invalid"))?;
        if sha256(&manifest_bytes) != terminal.session_manifest_sha256 {
            return Err(EvidenceReadError::Conflicting(
                "Claude session manifest digest mismatch",
            ));
        }
        let observation = parse_json(
            std::str::from_utf8(&observation_bytes)
                .map_err(|_| EvidenceReadError::Conflicting("Claude observation text invalid"))?,
        )
        .map_err(|_| EvidenceReadError::Conflicting("Claude observation JSON invalid"))?;
        claim_evidence::validate_production_path_observation(&observation)
            .map_err(|_| EvidenceReadError::Conflicting("Claude observation contract invalid"))?;
        let observed_at = observation
            .get("observedAt")
            .and_then(|value| value.as_text().ok())
            .and_then(|value| value.to_string_checked().ok())
            .ok_or(EvidenceReadError::Conflicting(
                "Claude observation time absent",
            ))?;
        if observation
            != super::writer::observation(&prepared, &capture, &result, Some(&observed_at))?
        {
            return Err(EvidenceReadError::Conflicting(
                "Claude observation binding mismatch",
            ));
        }
        let checked = CheckedExecutionResult::from_verified_parts(
            reference.clone(),
            &result,
            prepared.provenance(),
            CheckedObservation::Present(observation),
            class,
        )?;
        if checked.raw_sha256() != capture.result_sha256 {
            return Err(EvidenceReadError::Conflicting(
                "Claude canonical result digest mismatch",
            ));
        }
        if store::anchor_ids(&self.root)? != self.anchors {
            return Err(EvidenceReadError::Conflicting(
                "Claude reader anchors changed during read",
            ));
        }
        Ok((checked, terminal, capture.clone(), ready, capture.session))
    }
}

impl ExecutionEvidenceOwner for QualifiedReader {
    fn read_result(
        &self,
        reference: &ExecutionEvidenceRef,
    ) -> Result<CheckedExecutionResult, EvidenceReadError> {
        self.read_checked(reference)
            .map(|(checked, _, _, _, _)| checked)
    }

    fn read_custody(
        &self,
        reference: &ExecutionEvidenceRef,
        binding: &ProductionPathAdmissionBinding,
        observation: Option<&JsValue>,
    ) -> Result<CustodyEvidence, EvidenceReadError> {
        if !self.selection.profile.installed() {
            return Err(EvidenceReadError::Unsupported(
                "offline Claude peer has no production custody",
            ));
        }
        binding
            .validate()
            .map_err(|_| EvidenceReadError::Conflicting("CE admission binding invalid"))?;
        let (checked, terminal, capture, _, _) = self.read_checked(reference)?;
        if !checked.provenance().binding.matches_admission(binding)
            || checked.claim_sha256() != binding.native_result_sha256
            || binding.session_id.as_deref() != Some(capture.observed_session_id.as_str())
        {
            return Err(EvidenceReadError::Conflicting(
                "Claude execution/CE binding mismatch",
            ));
        }
        let stored = match checked.observation() {
            CheckedObservation::Present(value) => value,
            _ => return Err(EvidenceReadError::Unresolved("Claude observation absent")),
        };
        if observation != Some(stored) {
            return Err(EvidenceReadError::Conflicting(
                "Claude observation differs from owned record",
            ));
        }
        Ok(CustodyEvidence {
            owner: store::OWNER.into(),
            reference: reference.custody_reference(),
            revision: reference.revision.clone(),
            sha256: reference.sha256.clone(),
            verifier: store::CLAUDE_VERIFIER.into(),
            profile: store::CLAUDE_PROFILE.into(),
            attempt_id: reference.attempt_id.clone(),
            native_result_sha256: checked.claim_sha256().into(),
            session_id: binding.session_id.clone(),
            observation_sha256: Some(terminal.observation_sha256),
            artifact_references: terminal.artifact_references,
        })
    }
}
