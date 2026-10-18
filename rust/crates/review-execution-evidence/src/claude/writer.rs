use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use claim_evidence::codec::canonical_json;
use tokio::process::Command;

use crate::contract::sha256;
use crate::process::{self, CleanupObservation, FailureKind, ObservedFailure};
use crate::store::{self, ReaderPin, Store};
use crate::{
    EvidenceReadError, ExecutionEvidenceRef, ExecutionPublication, OBSERVATION_MAX_BYTES,
    RESULT_MAX_BYTES, STREAM_MAX_BYTES, TERMINAL_MAX_BYTES,
};

use super::qualified::{InitialEntryPermit, Prepared, QualifiedScope, SessionEvolutionPermit};
use super::session::{
    RetainedReviewSession, SessionReady, capture_snapshot, create_live, decode_guard_export,
    import_guard_export, reserve_snapshot, verify_live, verify_snapshot,
};
use super::transport::validate_response;

use review_execution_custody::claude::ResourceSpawn;
use review_execution_custody::claude::record::{CompleteCapture, QualifiedTerminal, observation};
use slice_campaign::FreshInitialEntry;
#[cfg(feature = "test-support")]
use slice_campaign::OfflineInitialEntry;

pub struct QualifiedWriter {
    store: Store,
    scope: QualifiedScope,
    pin: ReaderPin,
}

impl QualifiedWriter {
    pub fn create(root: &Path, scope: QualifiedScope) -> Result<Self, EvidenceReadError> {
        let pin = scope.reader_selection()?.pin;
        scope.preflight()?;
        let store = Store::create(root, &pin)?;
        fs::set_permissions(root, fs::Permissions::from_mode(0o700))
            .map_err(|_| EvidenceReadError::Unresolved("Claude custody permissions failed"))?;
        Ok(Self { store, scope, pin })
    }

    /// Reopening never restores an entry token and contains no launch path.
    pub fn resume(root: &Path, scope: QualifiedScope) -> Result<Self, EvidenceReadError> {
        let pin = scope.reader_selection()?.pin;
        let store = Store::resume(root, &pin)?;
        Ok(Self { store, scope, pin })
    }

    pub fn root(&self) -> &Path {
        &self.store.root
    }
    pub fn reader_pin(&self) -> &ReaderPin {
        &self.pin
    }

    /// Record a later owner-authorized session snapshot. The permit has no
    /// production constructor in RP1, and this method never launches a peer.
    pub fn record_session_evolution(
        &mut self,
        prior: RetainedReviewSession,
        permit: SessionEvolutionPermit,
    ) -> Result<RetainedReviewSession, EvidenceReadError> {
        let next_attempt = permit.consume(&self.scope, &prior)?;
        prior.verify_predecessor()?;
        let locator = prior.locator();
        if prior.root() != self.store.root
            || locator.root_id != self.pin.root_id
            || locator.profile != self.pin.profile
            || locator.session_id != self.scope.prepared.requested_session_id
            || next_attempt == locator.attempt_id
            || prior.chain().len() >= 64
        {
            return Err(EvidenceReadError::Conflicting(
                "successor session owner mismatch",
            ));
        }
        let initial = self.store.events(&self.scope.prepared.attempt_id)?;
        let terminal = initial
            .last()
            .filter(|event| event.stage == "terminal")
            .ok_or(EvidenceReadError::Unresolved(
                "initial Claude terminal absent",
            ))?;
        let reference = self.reference(&terminal.sha256);
        let reader =
            super::reader::QualifiedReader::open(&self.store.root, self.scope.reader_selection()?)?;
        let (ready, checked_chain) = reader.checked_chain(&reference, &locator.attempt_id)?;
        if checked_chain != prior.chain() || ready != prior.ready() {
            return Err(EvidenceReadError::Conflicting(
                "successor predecessor record changed",
            ));
        }
        if !self.store.events(&next_attempt)?.is_empty() {
            return Err(EvidenceReadError::Conflicting(
                "successor attempt already recorded",
            ));
        }
        self.store.reserve_terminal_capacity()?;
        reserve_snapshot(&self.store.root)?;
        let predecessor = prior.chain().last().unwrap();
        let manifest = if let Some(source) = &self.scope.selection.credential {
            let access = source
                .read_access_only_exact(
                    now_millis(),
                    self.scope.selection.horizon.as_millis() as i64,
                )
                .map_err(|_| {
                    EvidenceReadError::Unresolved("successor credential projection unavailable")
                })?;
            access.with_token(|token| {
                capture_snapshot(
                    &self.store.root,
                    &ready,
                    &next_attempt,
                    &locator.session_id,
                    Some(token),
                    Some(predecessor),
                )
            })?
        } else {
            capture_snapshot(
                &self.store.root,
                &ready,
                &next_attempt,
                &locator.session_id,
                None,
                Some(predecessor),
            )?
        };
        let bytes = serde_json::to_vec(&manifest).map_err(|_| {
            EvidenceReadError::Conflicting("successor manifest serialization failed")
        })?;
        self.store
            .append(&next_attempt, "session_evolution", &bytes, &[])?;
        reader.retained_session_after(&reference, &next_attempt)
    }

    fn reference(&self, digest: &str) -> ExecutionEvidenceRef {
        ExecutionEvidenceRef {
            schema_version: 1,
            owner: store::OWNER.into(),
            root_id: self.pin.root_id.clone(),
            profile: self.pin.profile.clone(),
            attempt_id: self.scope.prepared.attempt_id.clone(),
            record_id: "terminal".into(),
            revision: "1".into(),
            sha256: digest.into(),
        }
    }

    pub fn reconcile(&mut self) -> Result<ExecutionPublication, EvidenceReadError> {
        let attempt = &self.scope.prepared.attempt_id;
        let events = self.store.events(attempt)?;
        let Some(last) = events.last() else {
            return Ok(ExecutionPublication::Absent);
        };
        match last.stage.as_str() {
            "pre_entry_refusal" => Ok(ExecutionPublication::DefinitePreEntryFailure),
            "terminal" => {
                let reference = self.reference(&last.sha256);
                let selection = self.scope.reader_selection()?;
                let reader = super::reader::QualifiedReader::open(&self.store.root, selection)?;
                crate::ExecutionEvidenceOwner::read_result(&reader, &reference)?;
                Ok(ExecutionPublication::Replayed(reference))
            }
            "capture_complete" => self.finish_from_capture(),
            _ => Ok(ExecutionPublication::Unresolved),
        }
    }

    fn fail_capture(
        &mut self,
        failure: &ObservedFailure,
    ) -> Result<ExecutionPublication, EvidenceReadError> {
        let bytes = serde_json::to_vec(failure)
            .map_err(|_| EvidenceReadError::Conflicting("Claude failure serialization failed"))?;
        self.store.append(
            &self.scope.prepared.attempt_id,
            "capture_failure",
            &bytes,
            &[],
        )?;
        fault_cut("capture_failure");
        Ok(ExecutionPublication::Unresolved)
    }

    fn validate_and_consume_entry(
        &self,
        binding: &slice_campaign::InitialEntryBinding,
        installed: bool,
    ) -> Result<InitialEntryPermit, EvidenceReadError> {
        let prepared = &self.scope.prepared;
        let profile = &self.scope.profile;
        let selection = &self.scope.selection;
        let applied = self
            .scope
            .applied_request
            .as_ref()
            .ok_or(EvidenceReadError::Conflicting(
                "Applied campaign request absent",
            ))?;
        if profile.installed() != installed
            || prepared.provenance_class
                != if installed {
                    "qualified_provider"
                } else {
                    "offline_peer"
                }
            || binding.resource_policy()
                != self
                    .scope
                    .boundary
                    .as_ref()
                    .map_or(super::session::STRICT_POLICY, |boundary| {
                        boundary.resource_policy()
                    })
            || self.scope.prepared.resource_policy.as_deref()
                != (binding.resource_policy() == super::session::COMBINED_1G_POLICY)
                    .then_some(super::session::COMBINED_1G_POLICY)
            || binding.campaign_root_id() != prepared.campaign_root_id
            || binding.obligation_id() != prepared.obligation_id
            || binding.request_digest() != prepared.prepared_request_sha256
            || binding.dispatched_request() != &applied.request
            || binding.prompt_template_sha256() != applied.prompt_template_sha256
            || super::qualified::compose_applied_initial_prompt(
                &applied.template,
                &applied.request,
            )? != selection.prompt
            || crate::contract::sha256(&applied.template) != applied.prompt_template_sha256
            || binding.attempt_horizon_millis() != prepared.attempt_horizon_millis
            || binding.review_episode_id() != prepared.review_episode_id
            || binding.dispatch_operation_id() != prepared.dispatch_operation_id
            || binding.dispatch_revision() != prepared.dispatch_revision
            || binding.attempt_id() != prepared.attempt_id
            || binding.session_id() != prepared.requested_session_id
            || binding.candidate_tree() != prepared.candidate_tree
            || self.scope.applied_physical_profile_digest.as_deref()
                != Some(binding.physical_profile_digest())
            || binding.selection_digest() != prepared.selection_digest
            || binding.claude_profile_digest()
                != prepared
                    .execution_profile_digest
                    .as_deref()
                    .unwrap_or(&prepared.profile_digest)
            || binding.claude_profile_digest() != profile.profile_digest()
            || binding.program_sha256() != prepared.executable_sha256
            || binding.guard_sha256() != prepared.guard_sha256.as_deref().unwrap_or("")
            || binding.cbm_store_sha256() != prepared.cbm_store_sha256
            || binding.catalog_sha256() != prepared.catalog_sha256
            || binding.requested_model() != prepared.requested_model
            || binding.credential_source_metadata_sha256()
                != selection
                    .credential
                    .as_ref()
                    .map(|source| source.metadata_digest())
                    .unwrap_or_default()
            || binding.acceptance_reference().is_empty()
            || !binding
                .acceptance_digest()
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
            || binding.acceptance_digest().len() != 64
            || binding.dispatch_receipt_digest().len() != 64
            || !binding
                .dispatch_receipt_digest()
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
            || (installed && selection.offline_witness.is_some())
        {
            return Err(EvidenceReadError::Conflicting(
                "Applied initial entry/scope mismatch",
            ));
        }
        self.scope.preflight()?;
        Ok(InitialEntryPermit::applied(&self.scope, !installed))
    }

    /// The campaign-owned fresh Applied capability is consumed before E1
    /// creates its private one-time launch permit.
    pub async fn execute_applied_initial(
        &mut self,
        entry: FreshInitialEntry,
    ) -> Result<ExecutionPublication, EvidenceReadError> {
        let permit = self.validate_and_consume_entry(entry.binding(), true)?;
        self.execute_initial(permit).await
    }

    #[cfg(feature = "test-support")]
    pub async fn execute_offline_applied(
        &mut self,
        entry: OfflineInitialEntry,
    ) -> Result<ExecutionPublication, EvidenceReadError> {
        let permit = self.validate_and_consume_entry(entry.binding(), false)?;
        self.execute_initial(permit).await
    }

    pub async fn execute_initial(
        &mut self,
        permit: InitialEntryPermit,
    ) -> Result<ExecutionPublication, EvidenceReadError> {
        permit.consume(&self.scope)?;
        let attempt = self.scope.prepared.attempt_id.clone();
        if !self.store.events(&attempt)?.is_empty() {
            return self.reconcile();
        }
        let ready = create_live(&self.store.root)?;
        let prepared_bytes = serde_json::to_vec(&self.scope.prepared).map_err(|_| {
            EvidenceReadError::Conflicting("Claude preparation serialization failed")
        })?;
        let subject_bytes = canonical_json(&self.scope.selection.subject)
            .map_err(|_| EvidenceReadError::Conflicting("Claude subject serialization failed"))?
            .into_bytes();
        self.store.append(
            &attempt,
            "prepared",
            &prepared_bytes,
            &[
                ("prompt", &self.scope.selection.prompt),
                ("subject", &subject_bytes),
            ],
        )?;
        fault_cut("prepared");
        let ready_bytes = serde_json::to_vec(&ready)
            .map_err(|_| EvidenceReadError::Conflicting("session identity serialization failed"))?;
        self.store
            .append(&attempt, "session_ready", &ready_bytes, &[])?;
        fault_cut("session_ready");
        if self.scope.preflight().is_err() {
            self.store
                .append(&attempt, "pre_entry_refusal", b"preflight drift", &[])?;
            fault_cut("pre_entry_refusal");
            return Ok(ExecutionPublication::DefinitePreEntryFailure);
        }
        let credential = if let Some(source) = &self.scope.selection.credential {
            match source.read_access_only_exact(
                now_millis(),
                self.scope.selection.horizon.as_millis() as i64,
            ) {
                Ok(access) => Some(access),
                Err(_) => {
                    self.store.append(
                        &attempt,
                        "pre_entry_refusal",
                        b"credential projection unavailable",
                        &[],
                    )?;
                    fault_cut("pre_entry_refusal");
                    return Ok(ExecutionPublication::DefinitePreEntryFailure);
                }
            }
        } else {
            None
        };
        self.store.reserve_terminal_capacity()?;
        self.store.append(
            &attempt,
            "launch_intent",
            if self.scope.prepared.schema_version == 2 {
                b"Claude initial entry v2"
            } else {
                b"Claude initial entry v1"
            },
            &[],
        )?;
        fault_cut("launch_intent");
        // A second preflight runs after intent and immediately before the launch
        // syscall. Failure here is unresolved unless local non-entry is proven.
        if self.scope.preflight().is_err() || verify_live(&self.store.root, &ready).is_err() {
            self.store.append(
                &attempt,
                "pre_entry_refusal",
                b"pre-launch boundary drift",
                &[],
            )?;
            fault_cut("pre_entry_refusal");
            return Ok(ExecutionPublication::DefinitePreEntryFailure);
        }
        let nonce = sha256(
            format!(
                "{}:{}",
                attempt,
                self.scope.prepared.execution_profile_digest()
            )
            .as_bytes(),
        );
        let (command, ready_line, relay, quota) = if let Some(boundary) = &self.scope.boundary {
            let launch = boundary
                .guarded_launch(
                    &self.scope.profile,
                    &self.store.root.join("session-live"),
                    &nonce,
                    &self.scope.arguments(),
                )
                .map_err(|_| EvidenceReadError::Unresolved("guarded Claude launch unavailable"))?;
            (
                launch.command,
                Some(launch.ready_line),
                Some(launch.relay),
                Some(launch.quota),
            )
        } else {
            let mut command = Command::new(self.scope.profile.program().path());
            command
                .args(self.scope.arguments())
                .current_dir(&self.scope.selection.candidate_path)
                .env_clear()
                .env("HOME", self.store.root.join("session-live"))
                .env("CLAUDE_CONFIG_DIR", self.store.root.join("session-live"))
                .env("WORK_ENGINE_OFFLINE_PEER", "1")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true);
            if let Some(witness) = &self.scope.selection.offline_witness {
                command.env("WORK_ENGINE_PEER_WITNESS", witness);
            }
            if let Some(access) = &credential {
                access.with_token(|token| {
                    command.env("CLAUDE_CODE_OAUTH_TOKEN", token);
                });
            }
            (command, None, None, None)
        };
        let mut running = match process::spawn_command(command) {
            Ok(running) => running,
            Err(_) => {
                self.store
                    .append(&attempt, "spawn_failure", b"local entry uncertain", &[])?;
                fault_cut("spawn_failure");
                return Ok(ExecutionPublication::Unresolved);
            }
        };
        let pid = running
            .pid()
            .ok_or(EvidenceReadError::Unresolved("Claude PID unavailable"))?;
        let spawn_payload = if self.scope.prepared.schema_version == 2 {
            let realized = quota.ok_or(EvidenceReadError::Conflicting(
                "Claude v2 quota realization absent",
            ))?;
            let event = ResourceSpawn {
                schema_version: 2,
                pid,
                resource_policy: super::session::COMBINED_1G_POLICY.into(),
                host_custody_charge: realized.host_charge,
                memory_max: realized.memory_max,
                page_size: realized.page_size,
                swap_max_zero: true,
                swap_current_zero: true,
            };
            event.validate(&self.scope.prepared)?;
            serde_json::to_vec(&event)
                .map_err(|_| EvidenceReadError::Conflicting("Claude v2 resource record invalid"))?
        } else {
            pid.to_be_bytes().to_vec()
        };
        self.store
            .append(&attempt, "spawn_outcome", &spawn_payload, &[])?;
        fault_cut("spawn_outcome");
        if let Some(expected) = &ready_line
            && let Err(failure) = running.await_ready(expected).await
        {
            return self.fail_capture(&failure);
        }
        // Prompt bytes cannot be delivered before the selected launch boundary
        // and live session identity have been checked.
        if self.scope.preflight().is_err() || verify_live(&self.store.root, &ready).is_err() {
            drop(running);
            return Ok(ExecutionPublication::Unresolved);
        }
        let input = if ready_line.is_some() {
            let mut frame = Vec::new();
            if let Some(access) = &credential {
                access.with_token(|token| {
                    frame.extend_from_slice(&(token.len() as u32).to_be_bytes());
                    frame.extend_from_slice(token.as_bytes());
                });
            } else {
                frame.extend_from_slice(&0u32.to_be_bytes());
            }
            frame.extend_from_slice(&(self.scope.selection.prompt.len() as u32).to_be_bytes());
            frame.extend_from_slice(&self.scope.selection.prompt);
            frame
        } else {
            self.scope.selection.prompt.clone()
        };
        let captured_result = if ready_line.is_some() {
            running
                .capture_guarded_with_horizon(
                    &input,
                    self.scope.selection.horizon,
                    Duration::from_secs(5),
                )
                .await
        } else {
            running
                .capture_with_horizon(&input, self.scope.selection.horizon, Duration::from_secs(5))
                .await
        };
        let mut captured = match captured_result {
            Ok(capture) => capture,
            Err(failure) => return self.fail_capture(&failure),
        };
        let cbm_observation = match relay {
            Some(relay) => match relay.finish() {
                Ok(value) => Some(serde_json::to_vec(&value).map_err(|_| {
                    EvidenceReadError::Conflicting("CBM observation serialization failed")
                })?),
                Err(_) => {
                    return self.fail_capture(&ObservedFailure::local(
                        FailureKind::MalformedResult,
                        Some(pid),
                        captured.stdout.len(),
                        captured.stderr.len(),
                        None,
                        CleanupObservation::NotNeeded,
                    ));
                }
            },
            None => None,
        };
        let raw_secret_exposed = credential.as_ref().is_some_and(|access| {
            access.with_token(|token| {
                captured
                    .stdout
                    .windows(token.len())
                    .any(|part| part == token.as_bytes())
                    || captured
                        .stderr
                        .windows(token.len())
                        .any(|part| part == token.as_bytes())
            })
        });
        let exported_files = if ready_line.is_some() {
            match decode_guard_export(&captured.stdout) {
                Ok(export) => {
                    captured.stdout = export.result;
                    Some(export.files)
                }
                Err(_) => {
                    return self.fail_capture(&ObservedFailure::local(
                        FailureKind::MalformedResult,
                        Some(pid),
                        captured.stdout.len(),
                        captured.stderr.len(),
                        None,
                        CleanupObservation::NotNeeded,
                    ));
                }
            }
        } else {
            None
        };
        if captured.stdout.len() > STREAM_MAX_BYTES || captured.stderr.len() > STREAM_MAX_BYTES {
            let kind = if captured.stdout.len() > STREAM_MAX_BYTES {
                FailureKind::StdoutOverflow
            } else {
                FailureKind::StderrOverflow
            };
            return self.fail_capture(&ObservedFailure::local(
                kind,
                Some(pid),
                captured.stdout.len(),
                captured.stderr.len(),
                None,
                CleanupObservation::NotNeeded,
            ));
        }
        let secret_exposed = raw_secret_exposed
            || credential.as_ref().is_some_and(|access| {
                access.with_token(|token| {
                    captured
                        .stdout
                        .windows(token.len())
                        .any(|part| part == token.as_bytes())
                        || captured
                            .stderr
                            .windows(token.len())
                            .any(|part| part == token.as_bytes())
                })
            });
        if secret_exposed {
            return self.fail_capture(&ObservedFailure::local(
                FailureKind::MalformedResult,
                Some(pid),
                captured.stdout.len(),
                captured.stderr.len(),
                None,
                CleanupObservation::NotNeeded,
            ));
        }
        let valid = match validate_response(
            &captured.stdout,
            &self.scope.profile,
            &self.scope.prepared.requested_session_id,
            &self.scope.selection.subject,
        ) {
            Ok(valid) => valid,
            Err(_) => {
                return self.fail_capture(&ObservedFailure::local(
                    FailureKind::MalformedResult,
                    Some(pid),
                    captured.stdout.len(),
                    captured.stderr.len(),
                    None,
                    CleanupObservation::NotNeeded,
                ));
            }
        };
        if valid.result.len() > RESULT_MAX_BYTES {
            return Err(EvidenceReadError::Capacity("Claude result exceeds limit"));
        }
        if let Some(files) = exported_files
            && import_guard_export(&self.store.root, &ready, &files).is_err()
        {
            return self.fail_capture(&ObservedFailure::local(
                FailureKind::ResultCapacity,
                Some(pid),
                captured.stdout.len(),
                captured.stderr.len(),
                None,
                CleanupObservation::NotNeeded,
            ));
        }
        let manifest = match credential.as_ref() {
            Some(access) => access.with_token(|token| {
                capture_snapshot(
                    &self.store.root,
                    &ready,
                    &attempt,
                    &valid.session_id,
                    Some(token),
                    None,
                )
            })?,
            None => capture_snapshot(
                &self.store.root,
                &ready,
                &attempt,
                &valid.session_id,
                None,
                None,
            )?,
        };
        let capture = CompleteCapture {
            schema_version: 1,
            pid,
            exit_code: captured.exit_code,
            response_sha256: sha256(&captured.stdout),
            stderr_sha256: sha256(&captured.stderr),
            result_sha256: sha256(&valid.result),
            observed_model: valid.observed_model,
            observed_session_id: valid.session_id,
            provider_reported_entry: valid.provider_reported_entry,
            prompt_submitted: true,
            local_reaped: true,
            guard_ready: ready_line.is_some(),
            cbm_observation_sha256: cbm_observation.as_ref().map(|bytes| sha256(bytes)),
            session: manifest,
        };
        let bytes = serde_json::to_vec(&capture)
            .map_err(|_| EvidenceReadError::Conflicting("Claude capture serialization failed"))?;
        let mut artifacts: Vec<(&str, &[u8])> = vec![
            ("response", &captured.stdout),
            ("stderr", &captured.stderr),
            ("result", &valid.result),
        ];
        if let Some(cbm) = &cbm_observation {
            artifacts.push(("cbm_observation", cbm));
        }
        self.store
            .append(&attempt, "capture_complete", &bytes, &artifacts)?;
        fault_cut("capture_complete");
        self.finish_from_capture()
    }

    fn finish_from_capture(&mut self) -> Result<ExecutionPublication, EvidenceReadError> {
        let attempt = self.scope.prepared.attempt_id.clone();
        let events = self.store.events(&attempt)?;
        let stages: Vec<_> = events.iter().map(|event| event.stage.as_str()).collect();
        if stages
            != [
                "prepared",
                "session_ready",
                "launch_intent",
                "spawn_outcome",
                "capture_complete",
            ]
        {
            return Ok(ExecutionPublication::Unresolved);
        }
        let prepared: Prepared = serde_json::from_slice(&events[0].payload)
            .map_err(|_| EvidenceReadError::Conflicting("Claude prepared record invalid"))?;
        if prepared.pin()? != self.pin || prepared != self.scope.prepared {
            return Err(EvidenceReadError::Conflicting(
                "Claude prepared binding changed",
            ));
        }
        let ready: SessionReady = serde_json::from_slice(&events[1].payload)
            .map_err(|_| EvidenceReadError::Conflicting("session identity record invalid"))?;
        let capture: CompleteCapture = serde_json::from_slice(&events[4].payload)
            .map_err(|_| EvidenceReadError::Conflicting("Claude capture record invalid"))?;
        let response =
            store::read_artifact(&self.store.conn, &attempt, "response", STREAM_MAX_BYTES)?;
        let stderr = store::read_artifact(&self.store.conn, &attempt, "stderr", STREAM_MAX_BYTES)?;
        let result = store::read_artifact(&self.store.conn, &attempt, "result", RESULT_MAX_BYTES)?;
        let cbm_observation = if let Some(digest) = &capture.cbm_observation_sha256 {
            let bytes = store::read_artifact(
                &self.store.conn,
                &attempt,
                "cbm_observation",
                OBSERVATION_MAX_BYTES,
            )?;
            if sha256(&bytes) != *digest {
                return Err(EvidenceReadError::Conflicting(
                    "CBM observation digest mismatch",
                ));
            }
            let value: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|_| EvidenceReadError::Conflicting("CBM observation invalid"))?;
            if value
                .get("after_index_status")
                .and_then(serde_json::Value::as_array)
                .is_none_or(|items| items.len() < 2)
                || value
                    .get("mcp_request_count")
                    .and_then(serde_json::Value::as_u64)
                    .is_none_or(|count| count == 0)
            {
                return Err(EvidenceReadError::Conflicting(
                    "CBM process observation incomplete",
                ));
            }
            Some(bytes)
        } else {
            None
        };
        let prompt = store::read_artifact(
            &self.store.conn,
            &attempt,
            "prompt",
            crate::REQUEST_MAX_BYTES,
        )?;
        let subject = store::read_artifact(
            &self.store.conn,
            &attempt,
            "subject",
            crate::REQUEST_MAX_BYTES,
        )?;
        let spawn_valid = if prepared.schema_version == 2 {
            serde_json::from_slice::<ResourceSpawn>(&events[3].payload)
                .ok()
                .is_some_and(|spawn| spawn.pid == capture.pid && spawn.validate(&prepared).is_ok())
        } else {
            events[3].payload == capture.pid.to_be_bytes()
        };
        if sha256(&response) != capture.response_sha256
            || sha256(&stderr) != capture.stderr_sha256
            || sha256(&result) != capture.result_sha256
            || sha256(&prompt) != prepared.prompt_sha256
            || sha256(&subject) != prepared.subject_sha256
            || !spawn_valid
            || capture.exit_code != 0
            || !capture.prompt_submitted
            || !capture.local_reaped
            || capture.observed_session_id != prepared.requested_session_id
            || capture.session.session_id != prepared.requested_session_id
            || capture.session.attempt_id != attempt
            || capture.session.predecessor_sha256.is_some()
            || capture.session.predecessor_attempt_id.is_some()
            || capture.guard_ready != self.scope.boundary.is_some()
            || capture.guard_ready != cbm_observation.is_some()
        {
            return Err(EvidenceReadError::Conflicting(
                "Claude capture binding invalid",
            ));
        }
        verify_live(&self.store.root, &ready)?;
        verify_snapshot(&self.store.root, &capture.session)?;
        let subject_value = super::qualified::parse_stored_subject(&subject)?;
        let validated = validate_response(
            &response,
            &self.scope.profile,
            &prepared.requested_session_id,
            &subject_value,
        )?;
        if validated.result != result || validated.observed_model != capture.observed_model {
            return Err(EvidenceReadError::Conflicting(
                "Claude captured result changed",
            ));
        }
        let observation = observation(&prepared, &capture, &result, None)?;
        let observation_bytes = canonical_json(&observation)
            .map_err(|_| EvidenceReadError::Conflicting("Claude observation serialization failed"))?
            .into_bytes();
        if observation_bytes.len() > OBSERVATION_MAX_BYTES {
            return Err(EvidenceReadError::Capacity(
                "Claude observation exceeds limit",
            ));
        }
        let manifest_bytes = serde_json::to_vec(&capture.session)
            .map_err(|_| EvidenceReadError::Conflicting("session manifest serialization failed"))?;
        let terminal = QualifiedTerminal {
            schema_version: 1,
            capture_event_sha256: events[4].sha256.clone(),
            result_sha256: capture.result_sha256,
            observation_sha256: sha256(&observation_bytes),
            session_manifest_sha256: sha256(&manifest_bytes),
            artifact_references: vec![
                format!(
                    "execution-artifact:{}:{}:response",
                    prepared.root_id, attempt
                ),
                format!("execution-artifact:{}:{}:stderr", prepared.root_id, attempt),
                format!(
                    "execution-artifact:{}:{}:session",
                    prepared.root_id, attempt
                ),
            ]
            .into_iter()
            .chain(
                cbm_observation
                    .as_ref()
                    .map(|_| format!("execution-artifact:{}:{}:cbm", prepared.root_id, attempt)),
            )
            .collect(),
        };
        let terminal_bytes = serde_json::to_vec(&terminal)
            .map_err(|_| EvidenceReadError::Conflicting("Claude terminal serialization failed"))?;
        if terminal_bytes.len() + observation_bytes.len() > TERMINAL_MAX_BYTES {
            return Err(EvidenceReadError::Capacity("Claude terminal exceeds limit"));
        }
        let digest = self.store.append(
            &attempt,
            "terminal",
            &terminal_bytes,
            &[("observation", &observation_bytes)],
        )?;
        fault_cut("terminal");
        Ok(ExecutionPublication::Committed(self.reference(&digest)))
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(feature = "test-faults")]
fn fault_cut(stage: &str) {
    if std::env::var("WORK_ENGINE_CLAUDE_FAULT_CUT")
        .ok()
        .as_deref()
        != Some(stage)
    {
        return;
    }
    let Ok(path) = std::env::var("WORK_ENGINE_CLAUDE_FAULT_READY") else {
        return;
    };
    let _ = std::fs::write(path, stage);
    std::thread::sleep(Duration::from_secs(15));
    std::process::abort();
}
#[cfg(not(feature = "test-faults"))]
fn fault_cut(_stage: &str) {}
