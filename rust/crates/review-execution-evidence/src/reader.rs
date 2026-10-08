use std::path::{Path, PathBuf};

use claim_evidence::codec::{JsValue, digest, parse_json};
use claim_evidence::{
    CustodyEvidence, ProductionPathAdmissionBinding, validate_production_path_observation,
};

use crate::contract::{record_digest, sha256};
use crate::producer::{CapturedRecord, StoredRequest, TerminalRecord};
use crate::store::{self, PROFILE, ReaderPin};
use crate::{
    CheckedExecutionResult, CheckedObservation, EvidenceClass, EvidenceReadError,
    EvidenceReaderProfile, ExecutionEvidenceOwner, ExecutionEvidenceRef, OBSERVATION_MAX_BYTES,
    REQUEST_MAX_BYTES, RESULT_MAX_BYTES, STREAM_MAX_BYTES,
};

/// Read-only installed reader. Each call reopens the anchored database, checks
/// its marker and rehashes the exact event chain and required BLOBs.
pub struct EvidenceReader {
    root: PathBuf,
    pin: ReaderPin,
    anchors: store::AnchorIds,
}

impl EvidenceReader {
    pub fn open(root: &Path, pin: ReaderPin) -> Result<Self, EvidenceReadError> {
        let (root, _, _) = store::open_reader(root, &pin)?;
        let anchors = store::anchor_ids(&root)?;
        Ok(Self { root, pin, anchors })
    }

    pub fn pin(&self) -> &ReaderPin {
        &self.pin
    }

    /// Read a bounded local failure fact after restart. This is diagnostic
    /// custody, not checked success or a campaign retry permit.
    pub fn read_local_failure(
        &self,
        attempt: &str,
    ) -> Result<Option<crate::ObservedFailure>, EvidenceReadError> {
        crate::contract::require_text(attempt)?;
        let (_, _, conn) = store::open_reader(&self.root, &self.pin)?;
        if store::anchor_ids(&self.root)? != self.anchors {
            return Err(EvidenceReadError::Conflicting(
                "reader root/database/fence replaced",
            ));
        }
        let events = store::read_events(&conn, attempt)?;
        let last = events.last().ok_or(EvidenceReadError::Absent)?;
        let failure = if last.stage == "capture_failure" {
            let failure: crate::ObservedFailure = serde_json::from_slice(&last.payload)
                .map_err(|_| EvidenceReadError::Conflicting("local failure record malformed"))?;
            if failure.schema_version != 1
                || failure.stdout_bytes > crate::STREAM_MAX_BYTES + 1
                || failure.stderr_bytes > crate::STREAM_MAX_BYTES + 1
            {
                return Err(EvidenceReadError::Conflicting(
                    "local failure record bounds invalid",
                ));
            }
            Some(failure)
        } else {
            None
        };
        if store::anchor_ids(&self.root)? != self.anchors {
            return Err(EvidenceReadError::Conflicting(
                "reader root/database/fence replaced during read",
            ));
        }
        Ok(failure)
    }

    fn read_checked(
        &self,
        reference: &ExecutionEvidenceRef,
    ) -> Result<(CheckedExecutionResult, TerminalRecord), EvidenceReadError> {
        reference.validate()?;
        EvidenceReaderProfile::ControlledProcess.require_class(EvidenceClass::ControlledProcess)?;
        if reference.owner != store::OWNER
            || reference.profile != PROFILE
            || reference.root_id != self.pin.root_id
            || reference.record_id != "terminal"
            || reference.revision != "1"
        {
            return Err(EvidenceReadError::Unsupported(
                "reference outside selected controlled root",
            ));
        }
        let (_, _, conn) = store::open_reader(&self.root, &self.pin)?;
        if store::anchor_ids(&self.root)? != self.anchors {
            return Err(EvidenceReadError::Conflicting(
                "reader root/database/fence replaced",
            ));
        }
        let artifact_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM artifacts WHERE attempt_id=?1",
                [&reference.attempt_id],
                |r| r.get(0),
            )
            .map_err(|_| EvidenceReadError::Unresolved("artifact inventory inaccessible"))?;
        let events = store::read_events(&conn, &reference.attempt_id)?;
        if events.is_empty() {
            return Err(EvidenceReadError::Absent);
        }
        let stages: Vec<&str> = events.iter().map(|event| event.stage.as_str()).collect();
        if stages
            != [
                "prepared",
                "launch_intent",
                "spawn_outcome",
                "result_captured",
                "terminal",
            ]
        {
            return Err(EvidenceReadError::Unresolved(
                "terminal custody incomplete or stage order invalid",
            ));
        }
        if artifact_count != 5 {
            return Err(EvidenceReadError::Conflicting(
                "artifact inventory differs from manifest",
            ));
        }
        if events[4].sha256 != reference.sha256 {
            return Err(EvidenceReadError::Conflicting(
                "terminal reference digest mismatch",
            ));
        }
        let request: StoredRequest = serde_json::from_slice(&events[0].payload)
            .map_err(|_| EvidenceReadError::Conflicting("stored request invalid"))?;
        let captured: CapturedRecord = serde_json::from_slice(&events[3].payload)
            .map_err(|_| EvidenceReadError::Conflicting("captured result record invalid"))?;
        let terminal: TerminalRecord = serde_json::from_slice(&events[4].payload)
            .map_err(|_| EvidenceReadError::Conflicting("terminal manifest invalid"))?;
        if request.schema_version != 1
            || terminal.schema_version != 1
            || request.root_id != self.pin.root_id
            || request.attempt_id != reference.attempt_id
            || request.executable_sha256 != self.pin.executable_sha256
            || request.source_sha256 != self.pin.source_sha256
            || record_digest(&[b"controlled-scope-v1", &events[0].payload])
                != self.pin.configuration_sha256
        {
            return Err(EvidenceReadError::Conflicting(
                "stored request/root binding mismatch",
            ));
        }
        request.provenance().validate()?;
        let input =
            store::read_artifact(&conn, &reference.attempt_id, "request", REQUEST_MAX_BYTES)?;
        let request_value = parse_json(
            std::str::from_utf8(&input)
                .map_err(|_| EvidenceReadError::Conflicting("request bytes invalid"))?,
        )
        .map_err(|_| EvidenceReadError::Conflicting("request JSON invalid"))?;
        if sha256(&input) != request.request_raw_sha256
            || digest(&request_value)
                .map_err(|_| EvidenceReadError::Conflicting("request digest invalid"))?
                != request.request_claim_sha256
        {
            return Err(EvidenceReadError::Conflicting(
                "request artifact binding mismatch",
            ));
        }
        if events[2].payload != captured.pid.to_be_bytes()
            || captured.pid != terminal.pid
            || captured.exit_code != terminal.exit_code
            || terminal.exit_code != 0
            || terminal.session_id != request.execution_session_id
        {
            return Err(EvidenceReadError::Conflicting(
                "spawn/capture/terminal mismatch",
            ));
        }
        let result =
            store::read_artifact(&conn, &reference.attempt_id, "result", RESULT_MAX_BYTES)?;
        let stdout =
            store::read_artifact(&conn, &reference.attempt_id, "stdout", STREAM_MAX_BYTES)?;
        let stderr =
            store::read_artifact(&conn, &reference.attempt_id, "stderr", STREAM_MAX_BYTES)?;
        let observation_bytes = store::read_artifact(
            &conn,
            &reference.attempt_id,
            "observation",
            OBSERVATION_MAX_BYTES,
        )?;
        if result != stdout
            || sha256(&result) != terminal.raw_result_sha256
            || sha256(&stdout) != terminal.stdout_sha256
            || sha256(&stderr) != terminal.stderr_sha256
            || captured.stdout_sha256 != terminal.stdout_sha256
            || captured.stderr_sha256 != terminal.stderr_sha256
            || terminal.transport_sha256 != terminal.stdout_sha256
        {
            return Err(EvidenceReadError::Conflicting(
                "terminal artifact digest mismatch",
            ));
        }
        let observation = parse_json(
            std::str::from_utf8(&observation_bytes)
                .map_err(|_| EvidenceReadError::Conflicting("observation bytes invalid"))?,
        )
        .map_err(|_| EvidenceReadError::Conflicting("observation JSON invalid"))?;
        validate_production_path_observation(&observation)
            .map_err(|_| EvidenceReadError::Conflicting("observation schema invalid"))?;
        if digest(&observation)
            .map_err(|_| EvidenceReadError::Conflicting("observation digest invalid"))?
            != terminal.observation_sha256
        {
            return Err(EvidenceReadError::Conflicting(
                "observation digest mismatch",
            ));
        }
        let expected_refs = ["stdout", "stderr"]
            .iter()
            .map(|kind| {
                format!(
                    "execution-artifact:{}:{}:{}",
                    self.pin.root_id, reference.attempt_id, kind
                )
            })
            .collect::<Vec<_>>();
        if terminal.artifact_references != expected_refs {
            return Err(EvidenceReadError::Conflicting(
                "terminal artifact references mismatch",
            ));
        }
        let obs_artifacts = observation
            .get("artifacts")
            .and_then(|v| v.as_array().ok())
            .ok_or(EvidenceReadError::Conflicting(
                "observation artifacts invalid",
            ))?;
        let obs_refs = obs_artifacts
            .iter()
            .map(|value| {
                value
                    .get("reference")
                    .and_then(|v| v.as_text().ok())
                    .and_then(|v| v.to_string_checked().ok())
                    .ok_or(EvidenceReadError::Conflicting(
                        "observation artifact reference invalid",
                    ))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if obs_refs != expected_refs {
            return Err(EvidenceReadError::Conflicting(
                "observation artifact set mismatch",
            ));
        }
        for (artifact, expected_digest) in obs_artifacts
            .iter()
            .zip([&terminal.stdout_sha256, &terminal.stderr_sha256])
        {
            let actual = artifact
                .get("digest")
                .and_then(|v| v.as_text().ok())
                .and_then(|v| v.to_string_checked().ok())
                .ok_or(EvidenceReadError::Conflicting(
                    "observation artifact digest invalid",
                ))?;
            if &actual != expected_digest {
                return Err(EvidenceReadError::Conflicting(
                    "observation artifact digest mismatch",
                ));
            }
        }
        let checked = CheckedExecutionResult::from_verified_parts(
            reference.clone(),
            &result,
            request.provenance(),
            CheckedObservation::Present(observation),
            EvidenceClass::ControlledProcess,
        )?;
        if checked.claim_sha256() != terminal.claim_result_sha256
            || checked.raw_sha256() != terminal.raw_result_sha256
        {
            return Err(EvidenceReadError::Conflicting("result digest mismatch"));
        }
        let observed = match checked.observation() {
            CheckedObservation::Present(value) => value,
            _ => unreachable!(),
        };
        let execution = observed
            .get("execution")
            .ok_or(EvidenceReadError::Conflicting(
                "observation execution missing",
            ))?;
        let result_digest = execution
            .get("resultDigest")
            .and_then(|v| v.as_text().ok())
            .and_then(|v| v.to_string_checked().ok())
            .ok_or(EvidenceReadError::Conflicting(
                "observation result digest invalid",
            ))?;
        if result_digest != checked.claim_sha256()
            || execution.get("attemptId") != Some(&JsValue::text(&reference.attempt_id))
            || observed.get("obligationId") != Some(&JsValue::text(&request.obligation_id))
            || observed
                .get("subject")
                .and_then(|v| v.get("reviewEpisodeId"))
                != Some(&JsValue::text(&request.review_episode_id))
            || observed.get("selection").and_then(|v| v.get("revision"))
                != Some(&JsValue::text(&request.selection_digest))
            || observed.get("selection").and_then(|v| v.get("id"))
                != Some(&JsValue::text(&request.selection_id))
            || observed.get("subject").and_then(|v| v.get("candidate"))
                != Some(&JsValue::object([
                    ("commit", JsValue::text(&request.candidate_commit)),
                    ("tree", JsValue::text(&request.candidate_tree)),
                    (
                        "patchIdentity",
                        JsValue::text(&request.candidate_patch_identity),
                    ),
                ]))
            || observed.get("continuity").and_then(|v| v.get("sessionId"))
                != Some(&JsValue::text(&request.execution_session_id))
        {
            return Err(EvidenceReadError::Conflicting(
                "observation stable binding mismatch",
            ));
        }
        if store::anchor_ids(&self.root)? != self.anchors {
            return Err(EvidenceReadError::Conflicting(
                "reader root/database/fence replaced during read",
            ));
        }
        Ok((checked, terminal))
    }
}

impl ExecutionEvidenceOwner for EvidenceReader {
    fn read_result(
        &self,
        reference: &ExecutionEvidenceRef,
    ) -> Result<CheckedExecutionResult, EvidenceReadError> {
        self.read_checked(reference).map(|(checked, _)| checked)
    }

    fn read_custody(
        &self,
        reference: &ExecutionEvidenceRef,
        binding: &ProductionPathAdmissionBinding,
        observation: Option<&JsValue>,
    ) -> Result<CustodyEvidence, EvidenceReadError> {
        binding
            .validate()
            .map_err(|_| EvidenceReadError::Conflicting("CE admission binding invalid"))?;
        let (checked, terminal) = self.read_checked(reference)?;
        if !checked.provenance().binding.matches_admission(binding)
            || checked.claim_sha256() != binding.native_result_sha256
            || binding.session_id.as_deref() != Some(terminal.session_id.as_str())
        {
            return Err(EvidenceReadError::Conflicting(
                "execution/CE binding mismatch",
            ));
        }
        let stored = match checked.observation() {
            CheckedObservation::Present(value) => value,
            _ => {
                return Err(EvidenceReadError::Unresolved(
                    "controlled observation unavailable",
                ));
            }
        };
        if observation != Some(stored) {
            return Err(EvidenceReadError::Conflicting(
                "observation differs from owned record",
            ));
        }
        Ok(CustodyEvidence {
            owner: store::OWNER.into(),
            reference: reference.custody_reference(),
            revision: reference.revision.clone(),
            sha256: reference.sha256.clone(),
            verifier: store::VERIFIER.into(),
            profile: store::PROFILE.into(),
            attempt_id: reference.attempt_id.clone(),
            native_result_sha256: checked.claim_sha256().into(),
            session_id: binding.session_id.clone(),
            observation_sha256: Some(terminal.observation_sha256),
            artifact_references: terminal.artifact_references,
        })
    }
}
