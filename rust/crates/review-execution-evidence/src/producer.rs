use std::fs;
use std::path::{Path, PathBuf};

use claim_evidence::codec::{JsValue, canonical_json, digest, parse_json};
use claim_evidence::validate_production_path_observation;
use serde::{Deserialize, Serialize};

use crate::contract::{record_digest, require_sha, require_text, sha256};
use crate::process;
use crate::store::{PROFILE, ReaderPin, Store};
use crate::{
    CheckedExecutionResult, CheckedObservation, EvidenceClass, EvidenceReadError, ExecutionBinding,
    ExecutionEvidenceRef, ExecutionProvenance, REQUEST_MAX_BYTES, RESULT_MAX_BYTES,
    TERMINAL_MAX_BYTES,
};

#[derive(Clone, Debug)]
pub struct ControlledScope {
    pub root_id: String,
    pub provenance: ExecutionProvenance,
    pub selection_id: String,
    pub candidate_commit: String,
    pub candidate_tree: String,
    pub candidate_patch_identity: String,
    pub request_bytes: Vec<u8>,
    pub executable: PathBuf,
    pub executable_sha256: String,
    pub arguments: Vec<String>,
    pub workspace: PathBuf,
    pub source_sha256: String,
}

impl ControlledScope {
    pub fn validate(&self) -> Result<(), EvidenceReadError> {
        self.provenance.validate()?;
        for value in [
            &self.root_id,
            &self.selection_id,
            &self.candidate_commit,
            &self.candidate_tree,
            &self.candidate_patch_identity,
        ] {
            require_text(value)?;
        }
        for argument in &self.arguments {
            require_text(argument)?;
        }
        for digest in [&self.executable_sha256, &self.source_sha256] {
            require_sha(digest)?;
        }
        if self.provenance.execution_session_id.is_none() {
            return Err(EvidenceReadError::Unsupported(
                "controlled process requires selected session",
            ));
        }
        if self.request_bytes.len() > REQUEST_MAX_BYTES {
            return Err(EvidenceReadError::Capacity("request exceeds limit"));
        }
        let text = std::str::from_utf8(&self.request_bytes)
            .map_err(|_| EvidenceReadError::Conflicting("request is not UTF-8"))?;
        let request =
            parse_json(text).map_err(|_| EvidenceReadError::Conflicting("request is not JSON"))?;
        if canonical_json(&request)
            .map_err(|_| EvidenceReadError::Conflicting("request canonicalization failed"))?
            .len()
            > REQUEST_MAX_BYTES
        {
            return Err(EvidenceReadError::Capacity(
                "canonical request exceeds limit",
            ));
        }
        if !self.executable.is_absolute() || !self.workspace.is_absolute() {
            return Err(EvidenceReadError::Unsupported(
                "controlled paths must be absolute",
            ));
        }
        for path in [&self.executable, &self.workspace] {
            if path
                .canonicalize()
                .map_err(|_| EvidenceReadError::Unresolved("controlled path inaccessible"))?
                != *path
            {
                return Err(EvidenceReadError::Conflicting(
                    "controlled path is not canonical",
                ));
            }
            let meta = fs::symlink_metadata(path)
                .map_err(|_| EvidenceReadError::Unresolved("controlled path inaccessible"))?;
            if meta.file_type().is_symlink() {
                return Err(EvidenceReadError::Conflicting("controlled path is symlink"));
            }
        }
        if !self.executable.is_file() || !self.workspace.is_dir() {
            return Err(EvidenceReadError::Conflicting(
                "controlled path type invalid",
            ));
        }
        Ok(())
    }

    pub fn reader_pin(&self) -> Result<ReaderPin, EvidenceReadError> {
        self.validate()?;
        Ok(ReaderPin {
            root_id: self.root_id.clone(),
            configuration_sha256: self.configuration_sha256()?,
            executable_sha256: self.executable_sha256.clone(),
            source_sha256: self.source_sha256.clone(),
            profile: PROFILE.into(),
        })
    }

    pub fn configuration_sha256(&self) -> Result<String, EvidenceReadError> {
        self.validate()?;
        let stored = StoredRequest::from_scope(self)?;
        let bytes = serde_json::to_vec(&stored)
            .map_err(|_| EvidenceReadError::Conflicting("scope serialization failed"))?;
        Ok(record_digest(&[b"controlled-scope-v1", &bytes]))
    }

    fn executable_still_matches(&self) -> Result<bool, EvidenceReadError> {
        let meta = fs::symlink_metadata(&self.executable)
            .map_err(|_| EvidenceReadError::Unresolved("executable inaccessible"))?;
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Ok(false);
        }
        let bytes = fs::read(&self.executable)
            .map_err(|_| EvidenceReadError::Unresolved("executable unreadable"))?;
        Ok(sha256(&bytes) == self.executable_sha256)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StoredRequest {
    pub schema_version: u32,
    pub root_id: String,
    pub campaign_root_id: String,
    pub obligation_id: String,
    pub candidate_digest: String,
    pub selection_digest: String,
    pub profile_digest: String,
    pub prepared_request_sha256: String,
    pub review_episode_id: String,
    pub attempt_id: String,
    pub dispatch_operation_id: String,
    pub dispatch_revision: String,
    pub execution_session_id: String,
    pub selection_id: String,
    pub candidate_commit: String,
    pub candidate_tree: String,
    pub candidate_patch_identity: String,
    pub request_raw_sha256: String,
    pub request_claim_sha256: String,
    pub executable_sha256: String,
    pub executable: String,
    pub arguments: Vec<String>,
    pub workspace: String,
    pub source_sha256: String,
}

impl StoredRequest {
    fn from_scope(scope: &ControlledScope) -> Result<Self, EvidenceReadError> {
        let b = &scope.provenance.binding;
        let request = parse_json(
            std::str::from_utf8(&scope.request_bytes)
                .map_err(|_| EvidenceReadError::Conflicting("request text invalid"))?,
        )
        .map_err(|_| EvidenceReadError::Conflicting("request JSON invalid"))?;
        Ok(Self {
            schema_version: 1,
            root_id: scope.root_id.clone(),
            campaign_root_id: b.campaign_root_id.clone(),
            obligation_id: b.obligation_id.clone(),
            candidate_digest: b.candidate_digest.clone(),
            selection_digest: b.selection_digest.clone(),
            profile_digest: b.profile_digest.clone(),
            prepared_request_sha256: b.prepared_request_sha256.clone(),
            review_episode_id: b.review_episode_id.clone(),
            attempt_id: b.attempt_id.clone(),
            dispatch_operation_id: scope.provenance.dispatch_operation_id.clone(),
            dispatch_revision: scope.provenance.dispatch_revision.clone(),
            execution_session_id: scope
                .provenance
                .execution_session_id
                .clone()
                .unwrap_or_default(),
            selection_id: scope.selection_id.clone(),
            candidate_commit: scope.candidate_commit.clone(),
            candidate_tree: scope.candidate_tree.clone(),
            candidate_patch_identity: scope.candidate_patch_identity.clone(),
            request_raw_sha256: sha256(&scope.request_bytes),
            request_claim_sha256: digest(&request)
                .map_err(|_| EvidenceReadError::Conflicting("request digest invalid"))?,
            executable_sha256: scope.executable_sha256.clone(),
            executable: scope
                .executable
                .to_str()
                .ok_or(EvidenceReadError::Unsupported("executable path not UTF-8"))?
                .into(),
            arguments: scope.arguments.clone(),
            workspace: scope
                .workspace
                .to_str()
                .ok_or(EvidenceReadError::Unsupported("workspace path not UTF-8"))?
                .into(),
            source_sha256: scope.source_sha256.clone(),
        })
    }

    pub fn provenance(&self) -> ExecutionProvenance {
        ExecutionProvenance {
            binding: ExecutionBinding {
                campaign_root_id: self.campaign_root_id.clone(),
                obligation_id: self.obligation_id.clone(),
                candidate_digest: self.candidate_digest.clone(),
                selection_digest: self.selection_digest.clone(),
                profile_digest: self.profile_digest.clone(),
                prepared_request_sha256: self.prepared_request_sha256.clone(),
                review_episode_id: self.review_episode_id.clone(),
                attempt_id: self.attempt_id.clone(),
            },
            dispatch_operation_id: self.dispatch_operation_id.clone(),
            dispatch_revision: self.dispatch_revision.clone(),
            execution_session_id: Some(self.execution_session_id.clone()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapturedRecord {
    pub pid: u32,
    pub exit_code: i32,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TerminalRecord {
    pub schema_version: u32,
    pub raw_result_sha256: String,
    pub claim_result_sha256: String,
    pub observation_sha256: String,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub transport_sha256: String,
    pub session_id: String,
    pub pid: u32,
    pub exit_code: i32,
    pub artifact_references: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionPublication {
    Absent,
    Committed(ExecutionEvidenceRef),
    Replayed(ExecutionEvidenceRef),
    DefinitePreEntryFailure,
    Unresolved,
}

/// Minted only after the immutable launch-intent transaction commits. The
/// private constructor and by-value process handoff prevent a caller from
/// selecting a child or replaying a launch ticket.
pub(crate) struct LaunchTicket {
    attempt_id: String,
}

impl LaunchTicket {
    fn after_durable_intent(attempt_id: &str) -> Self {
        Self {
            attempt_id: attempt_id.into(),
        }
    }
    pub(crate) fn matches(&self, scope: &ControlledScope) -> bool {
        self.attempt_id == scope.provenance.binding.attempt_id
    }
}

pub struct EvidenceWriter {
    store: Store,
    scope: ControlledScope,
    pin: ReaderPin,
}

impl EvidenceWriter {
    pub fn create(root: &Path, scope: ControlledScope) -> Result<Self, EvidenceReadError> {
        let pin = scope.reader_pin()?;
        if !scope.executable_still_matches()? {
            return Err(EvidenceReadError::Conflicting(
                "selected executable digest mismatch",
            ));
        }
        let store = Store::create(root, &pin)?;
        Ok(Self { store, scope, pin })
    }

    pub fn resume(root: &Path, scope: ControlledScope) -> Result<Self, EvidenceReadError> {
        let pin = scope.reader_pin()?;
        let store = Store::resume(root, &pin)?;
        Ok(Self { store, scope, pin })
    }

    pub fn reader_pin(&self) -> &ReaderPin {
        &self.pin
    }
    pub fn root(&self) -> &Path {
        &self.store.root
    }

    pub fn reconcile(&self) -> Result<ExecutionPublication, EvidenceReadError> {
        let events = self
            .store
            .events(&self.scope.provenance.binding.attempt_id)?;
        if events.is_empty() {
            return Ok(ExecutionPublication::Absent);
        }
        if let Some(last) = events.last() {
            if last.stage == "terminal" {
                let reference = self.reference(&last.sha256);
                let reader = crate::EvidenceReader::open(&self.store.root, self.pin.clone())?;
                crate::ExecutionEvidenceOwner::read_result(&reader, &reference)?;
                return Ok(ExecutionPublication::Replayed(reference));
            }
            if last.stage == "pre_entry_refusal" {
                return Ok(ExecutionPublication::DefinitePreEntryFailure);
            }
        }
        Ok(ExecutionPublication::Unresolved)
    }

    fn reference(&self, digest: &str) -> ExecutionEvidenceRef {
        ExecutionEvidenceRef {
            schema_version: 1,
            owner: crate::store::OWNER.into(),
            root_id: self.pin.root_id.clone(),
            profile: PROFILE.into(),
            attempt_id: self.scope.provenance.binding.attempt_id.clone(),
            record_id: "terminal".into(),
            revision: "1".into(),
            sha256: digest.into(),
        }
    }

    fn append_capture_failure(
        &mut self,
        attempt: &str,
        failure: &process::ObservedFailure,
    ) -> Result<(), EvidenceReadError> {
        let payload = serde_json::to_vec(failure)
            .map_err(|_| EvidenceReadError::Conflicting("failure serialization failed"))?;
        self.store
            .append(attempt, "capture_failure", &payload, &[])?;
        fault_cut("capture_failure");
        Ok(())
    }

    pub async fn execute(&mut self) -> Result<ExecutionPublication, EvidenceReadError> {
        let attempt = self.scope.provenance.binding.attempt_id.clone();
        if !self.store.events(&attempt)?.is_empty() {
            return self.reconcile();
        }
        let request = StoredRequest::from_scope(&self.scope)?;
        let request_payload = serde_json::to_vec(&request)
            .map_err(|_| EvidenceReadError::Conflicting("request serialization failed"))?;
        self.store.append(
            &attempt,
            "prepared",
            &request_payload,
            &[("request", &self.scope.request_bytes)],
        )?;
        fault_cut("prepared");
        // Only this checked pre-launch refusal proves local non-entry.
        if !self.scope.executable_still_matches()? {
            self.store.append(
                &attempt,
                "pre_entry_refusal",
                b"executable digest changed",
                &[],
            )?;
            fault_cut("pre_entry_refusal");
            return Ok(ExecutionPublication::DefinitePreEntryFailure);
        }
        self.store.reserve_terminal_capacity()?;
        self.store.append(
            &attempt,
            "launch_intent",
            b"controlled launch intent v1",
            &[],
        )?;
        fault_cut("launch_intent");
        let ticket = LaunchTicket::after_durable_intent(&attempt);
        let running = match process::spawn(&self.scope, ticket) {
            Ok(child) => child,
            Err(_) => {
                self.store.append(
                    &attempt,
                    "spawn_failure",
                    b"spawn failed; entry uncertain",
                    &[],
                )?;
                fault_cut("spawn_failure");
                return Ok(ExecutionPublication::Unresolved);
            }
        };
        let pid = running
            .pid()
            .ok_or(EvidenceReadError::Unresolved("spawned child PID missing"))?;
        self.store
            .append(&attempt, "spawn_outcome", &pid.to_be_bytes(), &[])?;
        fault_cut("spawn_outcome");
        let capture = match running.capture(&self.scope.request_bytes).await {
            Ok(capture) => capture,
            Err(failure) => {
                self.append_capture_failure(&attempt, &failure)?;
                return Ok(ExecutionPublication::Unresolved);
            }
        };
        if capture.stdout.len() > RESULT_MAX_BYTES {
            self.append_capture_failure(
                &attempt,
                &captured_failure(process::FailureKind::ResultCapacity, &capture),
            )?;
            return Ok(ExecutionPublication::Unresolved);
        }
        let value = std::str::from_utf8(&capture.stdout)
            .ok()
            .and_then(|s| parse_json(s).ok());
        if value.is_none() {
            self.append_capture_failure(
                &attempt,
                &captured_failure(process::FailureKind::MalformedResult, &capture),
            )?;
            return Ok(ExecutionPublication::Unresolved);
        }
        let checked = match CheckedExecutionResult::from_verified_parts(
            self.reference(&"0".repeat(64)),
            &capture.stdout,
            self.scope.provenance.clone(),
            CheckedObservation::Absent {
                reason: "observation pending".into(),
            },
            EvidenceClass::ControlledProcess,
        ) {
            Ok(checked) => checked,
            Err(EvidenceReadError::Capacity(_)) => {
                self.append_capture_failure(
                    &attempt,
                    &captured_failure(process::FailureKind::ResultCapacity, &capture),
                )?;
                return Ok(ExecutionPublication::Unresolved);
            }
            Err(_) => {
                self.append_capture_failure(
                    &attempt,
                    &captured_failure(process::FailureKind::MalformedResult, &capture),
                )?;
                return Ok(ExecutionPublication::Unresolved);
            }
        };
        let captured = CapturedRecord {
            pid: capture.pid,
            exit_code: capture.exit_code,
            stdout_sha256: sha256(&capture.stdout),
            stderr_sha256: sha256(&capture.stderr),
        };
        let captured_payload = serde_json::to_vec(&captured)
            .map_err(|_| EvidenceReadError::Conflicting("capture serialization failed"))?;
        self.store
            .append(&attempt, "result_captured", &captured_payload, &[])?;
        fault_cut("result_captured");
        let observation = build_observation(&self.scope, &captured, checked.claim_sha256())?;
        validate_production_path_observation(&observation)
            .map_err(|_| EvidenceReadError::Conflicting("built observation invalid"))?;
        let observation_bytes = canonical_json(&observation)
            .map_err(|_| EvidenceReadError::Conflicting("observation serialization failed"))?
            .into_bytes();
        if observation_bytes.len() > crate::OBSERVATION_MAX_BYTES {
            return Err(EvidenceReadError::Capacity("observation exceeds limit"));
        }
        let terminal = TerminalRecord {
            schema_version: 1,
            raw_result_sha256: sha256(&capture.stdout),
            claim_result_sha256: checked.claim_sha256().into(),
            observation_sha256: digest(&observation)
                .map_err(|_| EvidenceReadError::Conflicting("observation digest invalid"))?,
            stdout_sha256: captured.stdout_sha256.clone(),
            stderr_sha256: captured.stderr_sha256.clone(),
            transport_sha256: captured.stdout_sha256.clone(),
            session_id: self
                .scope
                .provenance
                .execution_session_id
                .clone()
                .unwrap_or_default(),
            pid: captured.pid,
            exit_code: captured.exit_code,
            artifact_references: artifact_references(&self.scope),
        };
        let terminal_payload = serde_json::to_vec(&terminal)
            .map_err(|_| EvidenceReadError::Conflicting("terminal serialization failed"))?;
        let total = terminal_payload.len()
            + capture.stdout.len() * 2
            + capture.stderr.len()
            + observation_bytes.len();
        if total > TERMINAL_MAX_BYTES {
            return Err(EvidenceReadError::Capacity("terminal exceeds limit"));
        }
        let terminal_digest = self.store.append(
            &attempt,
            "terminal",
            &terminal_payload,
            &[
                ("result", &capture.stdout),
                ("stdout", &capture.stdout),
                ("stderr", &capture.stderr),
                ("observation", &observation_bytes),
            ],
        )?;
        fault_cut("terminal");
        // A lost acknowledgement is reconciled by this exact terminal digest.
        Ok(ExecutionPublication::Committed(
            self.reference(&terminal_digest),
        ))
    }
}

fn captured_failure(
    kind: process::FailureKind,
    capture: &process::CapturedChild,
) -> process::ObservedFailure {
    process::ObservedFailure {
        schema_version: 1,
        kind,
        pid: Some(capture.pid),
        exit_code: Some(capture.exit_code),
        exit_signal: None,
        stdout_bytes: capture.stdout.len(),
        stderr_bytes: capture.stderr.len(),
        cleanup: process::CleanupObservation::NotNeeded,
    }
}

#[cfg(feature = "test-faults")]
fn fault_cut(stage: &str) {
    if std::env::var("WORK_ENGINE_EVIDENCE_FAULT_CUT")
        .ok()
        .as_deref()
        != Some(stage)
    {
        return;
    }
    let Ok(path) = std::env::var("WORK_ENGINE_EVIDENCE_FAULT_READY") else {
        return;
    };
    let _ = std::fs::write(path, stage);
    // The separate harness kills this process promptly. Expiry aborts rather
    // than allowing a paused process to continue an effect after a missed cut.
    std::thread::sleep(std::time::Duration::from_secs(15));
    std::process::abort();
}

#[cfg(not(feature = "test-faults"))]
fn fault_cut(_stage: &str) {}

fn artifact_references(scope: &ControlledScope) -> Vec<String> {
    ["stdout", "stderr"]
        .iter()
        .map(|kind| {
            format!(
                "execution-artifact:{}:{}:{}",
                scope.root_id, scope.provenance.binding.attempt_id, kind
            )
        })
        .collect()
}

fn build_observation(
    scope: &ControlledScope,
    capture: &CapturedRecord,
    result_digest: &str,
) -> Result<JsValue, EvidenceReadError> {
    let b = &scope.provenance.binding;
    let artifact_refs = artifact_references(scope);
    let mut value = JsValue::object([
        ("schema_version", JsValue::Number(2.0)),
        ("id", JsValue::text("pending")),
        (
            "event_identity",
            JsValue::text(&format!("controlled:{}:{}", scope.root_id, b.attempt_id)),
        ),
        ("kind", JsValue::text("production_path")),
        (
            "selection",
            JsValue::object([
                ("id", JsValue::text(&scope.selection_id)),
                ("revision", JsValue::text(&b.selection_digest)),
            ]),
        ),
        ("obligationId", JsValue::text(&b.obligation_id)),
        (
            "subject",
            JsValue::object([
                (
                    "candidate",
                    JsValue::object([
                        ("commit", JsValue::text(&scope.candidate_commit)),
                        ("tree", JsValue::text(&scope.candidate_tree)),
                        (
                            "patchIdentity",
                            JsValue::text(&scope.candidate_patch_identity),
                        ),
                    ]),
                ),
                ("reviewEpisodeId", JsValue::text(&b.review_episode_id)),
            ]),
        ),
        ("coveredState", JsValue::text("review-result")),
        (
            "execution",
            JsValue::object([
                ("attemptId", JsValue::text(&b.attempt_id)),
                ("resultDigest", JsValue::text(result_digest)),
            ]),
        ),
        (
            "realization",
            JsValue::object([
                ("requested", JsValue::text("controlled-child-v1")),
                ("observed", JsValue::text("controlled-child-v1")),
            ]),
        ),
        (
            "capabilityEnvelope",
            JsValue::object([
                ("capabilities", JsValue::Array(vec![])),
                // The controlled child retains its ordinary local process access.
                // This profile does not attest a realized read-only grant.
                ("mutationAuthorized", JsValue::Bool(true)),
            ]),
        ),
        (
            "continuity",
            JsValue::object([
                ("mode", JsValue::text("fresh_initial")),
                (
                    "sessionId",
                    JsValue::text(
                        scope
                            .provenance
                            .execution_session_id
                            .as_deref()
                            .unwrap_or(""),
                    ),
                ),
            ]),
        ),
        (
            "transport",
            JsValue::object([
                ("mechanism", JsValue::text("controlled-stdio-v1")),
                ("digest", JsValue::text(&capture.stdout_sha256)),
            ]),
        ),
        (
            "observer",
            JsValue::object([
                ("identity", JsValue::text("review-execution-evidence")),
                ("kind", JsValue::text("controlled-process-owner")),
            ]),
        ),
        ("observedAt", JsValue::text(&canonical_now_utc())),
        ("adapterVersion", JsValue::text("controlled-process-v1")),
        (
            "artifacts",
            JsValue::Array(vec![
                artifact_value(&artifact_refs[0], &capture.stdout_sha256),
                artifact_value(&artifact_refs[1], &capture.stderr_sha256),
            ]),
        ),
    ]);
    let without_id = match &value {
        JsValue::Object(fields) => {
            let mut fields = fields.clone();
            fields.remove(&"id".into());
            JsValue::Object(fields)
        }
        _ => unreachable!(),
    };
    let id = format!(
        "production-path-observation-v1@{}",
        digest(&without_id)
            .map_err(|_| EvidenceReadError::Conflicting("observation ID digest invalid"))?
    );
    if let JsValue::Object(fields) = &mut value {
        fields.insert("id".into(), JsValue::text(&id));
    }
    Ok(value)
}

fn artifact_value(reference: &str, digest: &str) -> JsValue {
    JsValue::object([
        ("owner", JsValue::text("review-execution-evidence")),
        ("reference", JsValue::text(reference)),
        ("digest", JsValue::text(digest)),
        ("status", JsValue::text("verified")),
    ])
}

fn canonical_now_utc() -> String {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let seconds = elapsed.as_secs();
    let days = (seconds / 86_400) as i64;
    let secs = seconds % 86_400;
    // Gregorian civil date from days since 1970-01-01.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += if month <= 2 { 1 } else { 0 };
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60,
        elapsed.subsec_millis()
    )
}
