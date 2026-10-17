use crate::provider::QualifiedInitialLaunch;
use crate::{Error, Result, Workflow, effect, fault_cut, refuse, startup::TrustedStartup};
use claim_evidence::{BootstrapFiles, RootIdentity};
use review_episode::{
    Application as EpisodeApplication,
    host_admission::NativeHostAdmission,
    in_process_admission::{EpisodeReadRequest, EpisodeReadTarget, TrustedEpisodeReadScope},
    protocol::{DEFAULT_RESPONSE_LIMIT, Request},
};
use review_episode_core::{
    codec::{JsString, JsValue as EValue, canonical_json, digest, field, parse_json},
    identity::Revision,
};
use review_episode_store::{
    EpisodeStore, NativeRootSelection, StoreOptions, init_native_root, read_native_selection,
};
#[cfg(feature = "test-support")]
use review_execution_evidence::CheckedObservation;
#[cfg(feature = "test-support")]
use review_execution_evidence::test_support::{
    ControlledFixtureOwner, FIXTURE_PROFILE, FixtureRecord,
};
use review_execution_evidence::{
    ControlledScope, EvidenceReader, EvidenceReaderProfile, EvidenceWriter, ExecutionBinding,
    ExecutionEvidenceOwner, ExecutionEvidenceRef, ExecutionProvenance, ExecutionPublication,
};
use serde_json::{Value, json};
use slice_campaign::{
    AdmitRequest, AdvancePhase, Campaign, CampaignRevision, CompletionSelection, Consequence,
    DispatchCommand, DispatchEffect, DispatchedRequest, InitialCompletionOwners,
    InitialCompletionProgress, InitialJoinSelection, InitialOutcomeState, InitialProgressState,
    NativeReviewRequestRef, Preparation, Snapshot, StageKind, StageProgress,
};
use std::{path::Path, sync::Arc};

pub(crate) fn initialize(startup: TrustedStartup) -> Result<Snapshot> {
    let d = &startup.document;
    // Refuse an unknown/partial existing episode owner before creating another root.
    if d.roots.episode.exists() {
        EpisodeStore::open_native(&d.roots.episode, StoreOptions::default())?;
    }
    let mut campaign = if d.roots.campaign.exists() {
        Campaign::open(startup.campaign_config()?)?
    } else {
        Campaign::initialize(startup.campaign_config()?)?
    };
    if d.roots.episode.exists() {
        EpisodeStore::open_native(&d.roots.episode, StoreOptions::default())?;
    } else {
        init_native_root(&d.roots.episode)?;
    }
    // Every resumed step uses the exact historical predecessor returned by replay.
    let admitted = effect(campaign.admit(
        &startup.id("admit"),
        AdmitRequest {
            identity: d.identity.clone(),
            repository: d.repository.clone(),
            workspace: d.workspace.clone(),
            accepted_boundary: d.accepted_boundary.clone(),
            baseline: d.baseline.clone(),
            expected_impact: None,
        },
    )?)?;
    let consequence = Consequence {
        owner: "slice-supervisor".into(),
        reference: d.accepted_boundary.reference.clone(),
        sha256: d.accepted_boundary.sha256.clone(),
    };
    let implementing = effect(campaign.advance(
        &d.identity,
        &admitted.revision,
        &startup.id("implementing"),
        AdvancePhase::Implementing,
        consequence.clone(),
    )?)?;
    let gate = effect(campaign.advance(
        &d.identity,
        &implementing.revision,
        &startup.id("gate"),
        AdvancePhase::GateReady,
        consequence.clone(),
    )?)?;
    let candidate = effect(campaign.bind_existing_candidate(
        &d.identity,
        &gate.revision,
        &startup.id("candidate"),
        &startup.candidate,
        &startup.physical_profile,
    )?)?;
    let ready = effect(campaign.advance(
        &d.identity,
        &candidate.revision,
        &startup.id("ready"),
        AdvancePhase::ReviewReady,
        consequence,
    )?)?;
    effect(campaign.bind_selection(
        &d.identity,
        &ready.revision,
        &startup.id("selection"),
        startup.selection.clone(),
    )?)?;
    campaign
        .read(&d.identity)?
        .ok_or_else(|| refuse("unresolved", "initialized campaign absent"))
}

// A process-local token, only constructed in the fresh applied-permit branch.
// It cannot be reconstructed from DispatchedRequest or a serialized reference.
struct InitialLaunch;
impl Workflow {
    pub(crate) fn command(&self) -> DispatchCommand {
        self.startup.dispatch_command()
    }

    pub(crate) async fn execute_initial(&mut self, expected: &CampaignRevision) -> Result<Value> {
        if self.startup.document.roots.evidence.exists()
            || self.startup.document.roots.claims.exists()
        {
            return Err(refuse(
                "unresolved",
                "existing execution/claims owners require recovery, not a fresh dispatch",
            ));
        }
        let state = self.snapshot()?;
        // Descriptive sizing only; this value cannot reach the private launch path.
        let sizing = NativeReviewRequestRef {
            root_id: self.campaign.root_id().into(),
            identity: self.startup.document.identity.clone(),
            obligation_id: self.startup.document.obligation_id.clone(),
            operation_id: self.startup.id("prepare"),
            kind: "initial".into(),
            prepared_revision: state.revision.clone(),
            request_digest: "0".repeat(64),
            selection: state
                .selection_ref
                .clone()
                .ok_or_else(|| refuse("scope", "campaign selection absent"))?,
            candidate: state
                .candidate
                .clone()
                .ok_or_else(|| refuse("scope", "campaign candidate absent"))?,
            profile_digest: state
                .physical_profile
                .as_ref()
                .and_then(|p| p["profile_digest"].as_str())
                .ok_or_else(|| refuse("scope", "physical profile absent"))?
                .into(),
        };
        if self.startup.qualified.is_some() {
            self.qualified_scope(&sizing, &"0".repeat(64))?;
        } else {
            self.scope(&sizing, &"0".repeat(64))?.validate()?;
        }
        fault_cut("before_dispatch");
        let d = &self.startup.document;
        let prepared = match self.campaign.prepare_initial(
            &d.identity,
            expected,
            &d.obligation_id,
            &self.startup.id("prepare"),
        )? {
            Preparation::Applied(value) => value,
            Preparation::Replayed { .. } => {
                return Err(refuse(
                    "unresolved",
                    "preparation replay is not a launch capability; recover original operation",
                ));
            }
            Preparation::NoEffect(reason) => return Err(refuse("no_effect", reason)),
            Preparation::OutcomeUnknown(locator) => {
                return Err(Error {
                    kind: "outcome_unknown",
                    message: "preparation outcome unknown".into(),
                    owner: Some(serde_json::to_value(locator)?),
                });
            }
        };
        fault_cut("after_preparation");
        let prepared_request = prepared.request.clone();
        let command = self.command();
        let (permit, receipt) = match self.campaign.authorize_dispatch(
            prepared.handle,
            &self.startup.id("dispatch"),
            command.clone(),
        )? {
            DispatchEffect::Applied { permit, receipt } => (permit, receipt),
            DispatchEffect::Replayed { .. } => {
                return Err(refuse("unresolved", "dispatch replay cannot launch"));
            }
            DispatchEffect::NoEffect(reason) => return Err(refuse("no_effect", reason)),
            DispatchEffect::OutcomeUnknown(locator) => {
                return Err(Error {
                    kind: "outcome_unknown",
                    message: "dispatch outcome unknown".into(),
                    owner: Some(serde_json::to_value(locator)?),
                });
            }
        };
        fault_cut("after_dispatch");
        let (dispatched, qualified_launch) = if let Some(runtime) = &self.startup.qualified {
            let launch = if runtime.installed() {
                QualifiedInitialLaunch::Installed(self.campaign.consume_qualified_initial(permit)?)
            } else {
                #[cfg(feature = "test-support")]
                {
                    QualifiedInitialLaunch::Offline(self.campaign.consume_offline_initial(permit)?)
                }
                #[cfg(not(feature = "test-support"))]
                {
                    return Err(refuse("unsupported", "offline initial route unavailable"));
                }
            };
            (
                DispatchedRequest {
                    request: prepared_request,
                    command: command.clone(),
                },
                Some(launch),
            )
        } else {
            (self.campaign.consume_dispatch_permit(permit)?, None)
        };
        if dispatched.command != command {
            return Err(refuse("conflict", "consumed dispatch differs"));
        }
        let launch = InitialLaunch;
        let native = read_native_selection(&self.startup.document.roots.episode)?;
        let begin = self.begin_request(&native);
        self.write_episode(&native, &begin, None)?;
        let content = EValue::object([
            ("action", EValue::text("begin")),
            ("unresolvedQuestions", EValue::Array(vec![])),
        ]);
        let target = EpisodeReadTarget::Transition {
            transition_id: JsString::new(&self.startup.id("begin")),
            content_digest: Revision(digest(&content)),
        };
        let read_scope =
            TrustedEpisodeReadScope::new(self.startup.authority.clone(), vec![target])?;
        let mut reader = EpisodeApplication::open_read_only(
            Path::new(&native.root),
            native.clone(),
            read_scope,
            StoreOptions::default(),
            DEFAULT_RESPONSE_LIMIT,
        )?;
        let observed = reader.read_checked(&EpisodeReadRequest::Transition {
            identity: self.startup.authority.identity().clone(),
            transition_id: JsString::new(&self.startup.id("begin")),
            content_digest: Revision(digest(&content)),
        })?;
        if observed.resolved_revision().is_none() {
            return Err(refuse(
                "unresolved",
                "episode begin readback absent before launch",
            ));
        }
        drop(reader);
        fault_cut("after_episode_begin");
        let reference = if let Some(entry) = qualified_launch {
            let scope =
                self.qualified_scope(&dispatched.request, receipt.result_revision.as_str())?;
            #[cfg(feature = "test-support")]
            let scope = {
                let mut scope = scope;
                if let Some(field) = std::env::var_os("RP2_TEST_APPLIED_SCOPE_MUTATION") {
                    scope.test_mutate_offline_applied_scope(&field.to_string_lossy())?;
                }
                scope
            };
            self.launch_qualified(entry, scope).await?
        } else {
            let scope = self.scope(&dispatched.request, receipt.result_revision.as_str())?;
            self.launch(launch, scope).await?
        };
        self.join(reference)
    }
    async fn launch(
        &self,
        _launch: InitialLaunch,
        scope: ControlledScope,
    ) -> Result<ExecutionEvidenceRef> {
        let root = &self.startup.document.roots.evidence;
        // A fresh dispatch never adopts an existing E1 writer or calls execute on recovery.
        if root.exists() {
            return Err(refuse(
                "unresolved",
                "execution root already exists for fresh dispatch",
            ));
        }
        let mut writer = EvidenceWriter::create(root, scope)?;
        let publication = writer.execute().await?;
        drop(writer);
        fault_cut("after_e1_terminal");
        match publication {
            ExecutionPublication::Committed(reference) => Ok(reference),
            other => Err(refuse(
                "unresolved",
                format!("execution publication {other:?}; recover exact owner state"),
            )),
        }
    }
    pub(crate) fn scope(
        &self,
        request: &NativeReviewRequestRef,
        dispatch_revision: &str,
    ) -> Result<ControlledScope> {
        let d = &self.startup.document;
        let command = self.command();
        let bytes = self.startup.request_bytes(request)?;
        #[cfg(feature = "test-support")]
        let execution_profile_digest =
            std::env::var_os("RP2_TEST_SYNTHETIC_FIXTURE_JOIN").map(|_| "e".repeat(64));
        #[cfg(not(feature = "test-support"))]
        let execution_profile_digest = None;
        Ok(ControlledScope {
            root_id: d.peer.evidence_root_id.clone(),
            provenance: ExecutionProvenance {
                binding: ExecutionBinding {
                    campaign_root_id: self.campaign.root_id().into(),
                    obligation_id: d.obligation_id.clone(),
                    candidate_digest: request.candidate.receipt_sha256.clone(),
                    selection_digest: request.selection.campaign_digest.clone(),
                    profile_digest: request.profile_digest.clone(),
                    execution_profile_digest,
                    prepared_request_sha256: request.request_digest.clone(),
                    review_episode_id: command.episode_id,
                    attempt_id: d.identity.attempt_id.clone(),
                },
                dispatch_operation_id: self.startup.id("dispatch"),
                dispatch_revision: dispatch_revision.into(),
                execution_session_id: Some(d.peer.session.clone()),
            },
            selection_id: request.selection.selection_id.clone(),
            candidate_commit: request.candidate.commit.clone(),
            candidate_tree: request.candidate.tree.clone(),
            candidate_patch_identity: request.candidate.patch_identity.clone(),
            request_bytes: bytes,
            executable: d.peer.executable.path.clone(),
            executable_sha256: d.peer.executable.sha256.clone(),
            arguments: d.peer.arguments.clone(),
            workspace: d.workspace.clone(),
            source_sha256: d.peer.source.sha256.clone(),
        })
    }
    pub(crate) fn join(&mut self, reference: ExecutionEvidenceRef) -> Result<Value> {
        let d = self.startup.document.clone();
        let prepared = self
            .snapshot()?
            .obligations
            .into_iter()
            .find(|o| o.obligation_id == d.obligation_id)
            .and_then(|o| o.request)
            .ok_or_else(|| refuse("unresolved", "original prepared request absent"))?;
        let dispatch = self.dispatch_receipt(&prepared)?;
        #[allow(unused_mut)] // Test-support injects a separately classified synthetic owner.
        let (mut reader, expected_provenance, mut evidence_profile, mut evidence_record_profile): (
            Arc<dyn ExecutionEvidenceOwner>,
            ExecutionProvenance,
            EvidenceReaderProfile,
            &str,
        ) = if let Some(runtime) = &self.startup.qualified {
            let scope = self.qualified_scope(&prepared, dispatch.result_revision.as_str())?;
            let expected = scope.reader_selection()?;
            let reader = review_execution_evidence::claude::QualifiedReader::open(
                &d.roots.evidence,
                expected,
            )?;
            (
                Arc::new(reader),
                scope.provenance().clone(),
                EvidenceReaderProfile::QualifiedProvider,
                if runtime.installed() {
                    "claude-qualified-v1"
                } else {
                    "claude-offline-peer-v1"
                },
            )
        } else {
            let scope = self.scope(&prepared, dispatch.result_revision.as_str())?;
            let reader = EvidenceReader::open(&d.roots.evidence, scope.reader_pin()?)?;
            (
                Arc::new(reader),
                scope.provenance,
                EvidenceReaderProfile::ControlledProcess,
                "controlled-process-v1",
            )
        };
        #[allow(unused_mut)] // Test-support substitutes a separately classified fixture reference.
        let mut reference = reference;
        // This adapter exists only in test-support builds. It replaces a
        // checked controlled-process result with a separately classified
        // synthetic fixture record before entering the same J1/J2 join below.
        // The ordinary qualified/offline path still refuses fixture custody.
        #[cfg(feature = "test-support")]
        if std::env::var_os("RP2_TEST_SYNTHETIC_FIXTURE_JOIN").is_some() {
            if self.startup.qualified.is_some() {
                return Err(refuse("unsupported", "synthetic join is controlled-only"));
            }
            let original = reader.read_result(&reference)?;
            if expected_provenance
                .binding
                .execution_profile_digest
                .as_deref()
                == Some(expected_provenance.binding.profile_digest.as_str())
                || expected_provenance
                    .binding
                    .execution_profile_digest
                    .is_none()
            {
                return Err(refuse(
                    "scope",
                    "synthetic distinct profile identities absent",
                ));
            }
            let mut observation = original.observation().clone();
            if std::env::var_os("RP2_TEST_SYNTHETIC_ESTABLISHED").is_some() {
                let CheckedObservation::Present(value) = &observation else {
                    return Err(refuse("scope", "synthetic established observation absent"));
                };
                let mut json: Value =
                    serde_json::from_str(&claim_evidence::codec::canonical_json(value)?)?;
                json["coveredState"] = json!("initial review");
                json["realization"] = json!({"requested":"claude-sonnet-5",
                    "observed":"claude-sonnet-5"});
                json["capabilityEnvelope"]["mutationAuthorized"] = json!(false);
                json["transport"]["mechanism"] = json!("direct");
                json["observer"] = json!({"identity":"host","kind":"trusted"});
                json["adapterVersion"] = json!("v1");
                json.as_object_mut().unwrap().remove("id");
                let unsigned = claim_evidence::codec::parse_json(&json.to_string())?;
                json["id"] = json!(format!(
                    "production-path-observation-v1@{}",
                    claim_evidence::codec::digest(&unsigned)?
                ));
                observation = CheckedObservation::Present(claim_evidence::codec::parse_json(
                    &json.to_string(),
                )?);
            }
            let artifacts = match &observation {
                CheckedObservation::Present(value) => {
                    let json: Value =
                        serde_json::from_str(&claim_evidence::codec::canonical_json(value)?)?;
                    json["artifacts"]
                        .as_array()
                        .ok_or_else(|| refuse("conflict", "synthetic artifact list absent"))?
                        .iter()
                        .map(|artifact| {
                            artifact["reference"]
                                .as_str()
                                .map(str::to_owned)
                                .ok_or_else(|| {
                                    refuse("conflict", "synthetic artifact reference absent")
                                })
                        })
                        .collect::<Result<Vec<_>>>()?
                }
                CheckedObservation::Absent { .. } => Vec::new(),
            };
            let mut synthetic_provenance = original.provenance().clone();
            synthetic_provenance.binding.execution_profile_digest =
                expected_provenance.binding.execution_profile_digest.clone();
            let record = FixtureRecord::controlled(
                &d.peer.evidence_root_id,
                &format!("rp2-synthetic-{}", reference.record_id),
                &reference.revision,
                original.claim_canonical_bytes().to_vec(),
                synthetic_provenance,
                observation,
                artifacts,
            )?;
            reference = record.reference.clone();
            // The production comparison below must still match the selected
            // campaign physical identity and the distinct execution identity.
            reader = Arc::new(ControlledFixtureOwner::new(vec![record])?);
            evidence_profile = EvidenceReaderProfile::ControlledFixture;
            evidence_record_profile = FIXTURE_PROFILE;
        }
        let checked = reader.read_result(&reference)?;
        evidence_profile.require_class(checked.evidence_class())?;
        if checked.provenance().binding != expected_provenance.binding
            || checked.provenance().dispatch_revision != expected_provenance.dispatch_revision
            || checked.provenance().execution_session_id != expected_provenance.execution_session_id
        {
            return Err(refuse("conflict", "execution provenance differs"));
        }
        let native = read_native_selection(&d.roots.episode)?;
        let selection = InitialJoinSelection {
            claims_root: RootIdentity {
                absolute_path: d.roots.claims.clone(),
                root_id: d.claims.root_id.clone(),
                profile: "native-review-claims-v1".into(),
            },
            production_grant_id: d.claims.production_grant_id.clone(),
            production_grant_sha256: d.claims.production_grant_sha256.clone(),
            episode_selection: native.clone(),
            episode_authority: self.startup.authority.clone(),
            evidence_profile,
            evidence_owner: "review-execution-evidence".into(),
            evidence_root_id: d.peer.evidence_root_id.clone(),
            evidence_record_profile: evidence_record_profile.into(),
        };
        let files = BootstrapFiles {
            root: d.roots.claims.clone(),
            config: d.claims.configuration.path.clone(),
            source: d.claims.source.path.clone(),
            expected_config_sha256: d.claims.configuration.sha256.clone(),
            expected_source_sha256: d.claims.source.sha256.clone(),
        };
        let mut claims = if d.roots.claims.exists() {
            self.campaign
                .open_initial_claims(files.into(), selection, reader)?
        } else {
            self.campaign
                .initialize_initial_claims(files, selection, reader)?
        };
        if let Some(runtime) = &self.startup.qualified
            && runtime.installed()
        {
            let completion = CompletionSelection::new(
                claims.claims().root().clone(),
                runtime.completion_grant_id.clone(),
                runtime.completion_grant_sha256.clone(),
                runtime.completion_grant_id.clone(),
                runtime.completion_grant_sha256.clone(),
                runtime.completion_consumer.clone(),
                prepared.candidate.tree.clone(),
                runtime.completion_decision_scope.clone(),
                vec![runtime.completion_authority_source.clone()],
            )?;
            self.campaign.install_completion_selection(completion)?;
        }
        #[cfg(feature = "test-support")]
        if std::env::var_os("RP2_TEST_SYNTHETIC_FIXTURE_JOIN").is_some() {
            let configuration: Value =
                serde_json::from_slice(&std::fs::read(&d.claims.configuration.path)?)?;
            let grant = configuration["grants"]
                .as_array()
                .and_then(|grants| {
                    grants
                        .iter()
                        .find(|grant| grant["grant_id"] == "h1-completion")
                })
                .ok_or_else(|| refuse("scope", "synthetic completion grant absent"))?;
            let grant_sha = claim_evidence::codec::digest(&claim_evidence::codec::parse_json(
                &grant.to_string(),
            )?)?;
            let completion = CompletionSelection::new(
                claims.claims().root().clone(),
                "h1-completion".into(),
                grant_sha.clone(),
                "h1-completion".into(),
                grant_sha,
                "campaign".into(),
                prepared.candidate.tree.clone(),
                grant["decision_scope"]
                    .as_str()
                    .ok_or_else(|| refuse("scope", "synthetic decision scope absent"))?
                    .into(),
                vec![grant["authority_reference"].clone()],
            )?;
            self.campaign.install_completion_selection(completion)?;
        }
        let completion_id = self.startup.id("completion");
        let locator = if let Some(locator) = self.campaign.locate_initial_completion(
            &d.identity,
            &d.obligation_id,
            &completion_id,
        )? {
            locator
        } else {
            let current = self.snapshot()?;
            effect(self.campaign.prepare_initial_completion(
                &d.identity,
                &current.revision,
                &d.obligation_id,
                &completion_id,
                &reference,
            )?)?
            .locator
        };
        let progress = self.campaign.recover_initial_completion(
            &locator,
            &mut InitialCompletionOwners::new(&mut claims, None),
        )?;
        let registered_content = progress
            .stages
            .iter()
            .find(|s| s.kind == StageKind::EpisodeResultRead)
            .map(|s| {
                s.command["contentCanonicalJson"]
                    .as_str()
                    .ok_or_else(|| refuse("conflict", "registered result canonical content absent"))
            })
            .transpose()?;
        let content = if let Some(content) = registered_content {
            parse_json(content)?
        } else {
            if !matches!(
                progress.state,
                InitialProgressState::Pending {
                    stage: StageKind::EpisodeResultRead,
                    ..
                }
            ) {
                return Ok(progress_value(progress));
            }
            let admission = |kind| -> Result<Value> {
                let record = progress
                    .stages
                    .iter()
                    .find(|s| s.kind == kind)
                    .and_then(|s| match &s.progress {
                        StageProgress::Checked { exact_refs, .. } => exact_refs.first(),
                        _ => None,
                    })
                    .ok_or_else(|| refuse("unresolved", "checked claim admission absent"))?;
                let claim = &record["claimRevisionRef"];
                let body = json!({"schemaVersion":1,"claimRevision":claim["revision"],"establishment":record["establishmentRef"],"boundary":record["boundary"],"consumer":record["consumer"]});
                let hash = claim_evidence::codec::digest(&claim_evidence::codec::parse_json(
                    &body.to_string(),
                )?)?;
                Ok(
                    json!({"claimRevisionRef":record["claimRevisionRef"],"establishmentRef":record["establishmentRef"],"observationRef":record["observationRef"],"consumptionRef":{"owner":"slice-campaign","reference":format!("claim-consumption:{}",claim["reference"].as_str().ok_or_else(|| refuse("conflict", "claim reference absent"))?),"revision":claim["revision"],"sha256":hash,"freshness":"exact immutable revision"},"status":record["status"],"boundary":record["boundary"],"consumer":record["consumer"]}),
                )
            };
            let admissions = json!([
                admission(StageKind::AdmissionBuilder)?,
                admission(StageKind::AdmissionCampaign)?
            ]);
            let result = parse_json(
                std::str::from_utf8(checked.claim_canonical_bytes())
                    .map_err(|_| refuse("execution_evidence", "result not UTF-8"))?,
            )?;
            let content = EValue::object([
                ("action", EValue::text("record_result")),
                (
                    "payload",
                    EValue::object([
                        ("result", result),
                        ("unresolvedQuestions", EValue::Array(vec![])),
                        ("evidenceAdmissions", parse_json(&admissions.to_string())?),
                    ]),
                ),
            ]);
            effect(self.campaign.register_initial_episode_read(
                &d.identity,
                &progress.last_checked_revision,
                &completion_id,
                &self.startup.id("register-result"),
                &content,
            )?)?;
            content
        };
        self.ensure_result(&native, &content)?;
        fault_cut("after_episode_result");
        let target = EpisodeReadTarget::Transition {
            transition_id: JsString::new(&self.startup.id("result")),
            content_digest: Revision(digest(&content)),
        };
        let scope = TrustedEpisodeReadScope::new(self.startup.authority.clone(), vec![target])?;
        let mut episode = EpisodeApplication::open_read_only(
            Path::new(&native.root),
            native.clone(),
            scope,
            StoreOptions::default(),
            DEFAULT_RESPONSE_LIMIT,
        )?;
        let mut owners = InitialCompletionOwners::new(&mut claims, Some(&mut episode));
        let joined = self
            .campaign
            .recover_initial_completion(&locator, &mut owners)?;
        if !matches!(joined.state, InitialProgressState::OwnerReadsJoined) {
            return Ok(progress_value(joined));
        }
        fault_cut("after_j1_joined");
        let outcome_id = self.startup.id("outcome");
        let outcome = if let Some(locator) =
            self.campaign
                .locate_initial_outcome(&d.identity, &d.obligation_id, &outcome_id)?
        {
            self.campaign
                .recover_initial_outcome(&locator, &mut owners)?
        } else {
            self.campaign.finalize_initial_completion(
                &d.identity,
                &joined.last_checked_revision,
                &joined.locator,
                &outcome_id,
                &mut owners,
            )?
        };
        fault_cut("after_j2_outcome");
        let state = match &outcome.state {
            InitialOutcomeState::Committed { view, receipt } => {
                json!({"status":"committed","view":view,"receipt":receipt})
            }
            InitialOutcomeState::Pending { stage, reason } => {
                json!({"status":"pending","stage":stage,"reason":reason})
            }
            InitialOutcomeState::Conflicting { stage, reason } => {
                json!({"status":"conflicting","stage":stage,"reason":reason})
            }
            InitialOutcomeState::Unresolved {
                stage,
                owner_locator,
                reason,
            } => {
                json!({"status":"unresolved","stage":stage,"ownerLocator":owner_locator,"reason":reason})
            }
        };
        Ok(
            json!({"status":"initial_outcome","outcome":state,"locator":outcome.locator,"lastCheckedRevision":outcome.last_checked_revision,"stages":outcome.stages,"newlyObservedReceipts":outcome.newly_observed_receipts,"executionEvidence":{"schemaVersion":reference.schema_version,"owner":reference.owner,"rootId":reference.root_id,"profile":reference.profile,"attemptId":reference.attempt_id,"recordId":reference.record_id,"revision":reference.revision,"sha256":reference.sha256}}),
        )
    }
    fn begin_request(&self, selection: &NativeRootSelection) -> EValue {
        self.episode_request(
            selection,
            "begin",
            &self.startup.id("begin"),
            EValue::object([
                ("authority", self.startup.authority.value().clone()),
                ("transitionId", EValue::text(&self.startup.id("begin"))),
                ("unresolvedQuestions", EValue::Array(vec![])),
            ]),
        )
    }
    fn ensure_result(&self, selection: &NativeRootSelection, content: &EValue) -> Result<()> {
        let begin_content = EValue::object([
            ("action", EValue::text("begin")),
            ("unresolvedQuestions", EValue::Array(vec![])),
        ]);
        let begin_target = EpisodeReadTarget::Transition {
            transition_id: JsString::new(&self.startup.id("begin")),
            content_digest: Revision(digest(&begin_content)),
        };
        let result_target = EpisodeReadTarget::Transition {
            transition_id: JsString::new(&self.startup.id("result")),
            content_digest: Revision(digest(content)),
        };
        let scope = TrustedEpisodeReadScope::new(
            self.startup.authority.clone(),
            vec![begin_target, result_target],
        )?;
        let mut reader = EpisodeApplication::open_read_only(
            Path::new(&selection.root),
            selection.clone(),
            scope,
            StoreOptions::default(),
            DEFAULT_RESPONSE_LIMIT,
        )?;
        let read = |id: String, hash: String| EpisodeReadRequest::Transition {
            identity: self.startup.authority.identity().clone(),
            transition_id: JsString::new(&id),
            content_digest: Revision(hash),
        };
        let begin = reader.read_checked(&read(self.startup.id("begin"), digest(&begin_content)))?;
        let first = begin
            .resolved_revision()
            .ok_or_else(|| refuse("unresolved", "original episode begin absent"))?
            .0
            .clone();
        let result = reader.read_checked(&read(self.startup.id("result"), digest(content)))?;
        if result.resolved_revision().is_some() {
            return Ok(());
        }
        if result.observed_revision().map(|r| &r.0) != Some(&first) {
            return Err(refuse(
                "conflict",
                "episode advanced outside selected result transition",
            ));
        }
        drop(reader);
        let request = self.episode_request(
            selection,
            "transition",
            &self.startup.id("result"),
            EValue::object([
                ("authority", self.startup.authority.value().clone()),
                ("expectedRevision", EValue::text(&first)),
                ("transitionId", EValue::text(&self.startup.id("result"))),
                ("action", EValue::text("record_result")),
                ("payload", field(content, "payload")?.clone()),
            ]),
        );
        self.write_episode(selection, &request, Some(&first))?;
        Ok(())
    }
    fn episode_request(
        &self,
        selection: &NativeRootSelection,
        operation: &str,
        id: &str,
        args: EValue,
    ) -> EValue {
        EValue::object([
            ("version", EValue::Number(2.0)),
            ("domain", EValue::text("review-episode")),
            ("profile", EValue::text("native-host-v1")),
            ("requestId", EValue::text(id)),
            ("operation", EValue::text(operation)),
            ("grantId", EValue::text(id)),
            ("selectionRevision", EValue::text(&self.startup.digest)),
            (
                "rootSelectionDigest",
                EValue::text(&selection.selection_digest),
            ),
            ("args", args),
        ])
    }
    fn write_episode(
        &self,
        selection: &NativeRootSelection,
        request: &EValue,
        observed: Option<&str>,
    ) -> Result<()> {
        let args = field(request, "args")?;
        let payload = args.get("payload");
        let operation = field(request, "operation")?;
        let content = if operation == &EValue::text("begin") {
            EValue::object([
                ("action", EValue::text("begin")),
                (
                    "unresolvedQuestions",
                    field(args, "unresolvedQuestions")?.clone(),
                ),
            ])
        } else {
            EValue::object([
                ("action", field(args, "action")?.clone()),
                ("payload", field(args, "payload")?.clone()),
            ])
        };
        let optional =
            |value: Option<&EValue>| value.map_or(EValue::Null, |v| EValue::text(&digest(v)));
        let descriptor = EValue::object([
            ("schemaVersion", EValue::Number(1.0)),
            ("profile", EValue::text("native-host-v1")),
            ("root", EValue::text(&selection.root)),
            (
                "executableSha256",
                EValue::text(&selection.executable_sha256),
            ),
            (
                "rootSelectionDigest",
                EValue::text(&selection.selection_digest),
            ),
            ("requestId", field(request, "requestId")?.clone()),
            ("operation", operation.clone()),
            ("grantId", field(request, "grantId")?.clone()),
            (
                "selectionRevision",
                field(request, "selectionRevision")?.clone(),
            ),
            ("requestDigest", EValue::text(&digest(request))),
            (
                "identityKey",
                EValue::text(&self.startup.authority.identity().key().0),
            ),
            ("argsDigest", EValue::text(&digest(args))),
            ("authorityDigest", optional(args.get("authority"))),
            (
                "resultDigest",
                optional(payload.and_then(|p| p.get("result"))),
            ),
            (
                "evidenceDigest",
                optional(payload.and_then(|p| p.get("evidenceAdmissions"))),
            ),
            ("contentDigest", EValue::text(&digest(&content))),
            (
                "observedRevision",
                observed.map_or(EValue::Null, EValue::text),
            ),
            (
                "principal",
                EValue::object([
                    ("id", EValue::text("native-review-host")),
                    (
                        "identityKey",
                        EValue::text(&self.startup.authority.identity().key().0),
                    ),
                    ("access", EValue::text("write")),
                ]),
            ),
        ]);
        let admission = NativeHostAdmission::from_trusted_descriptor(
            selection.clone(),
            canonical_json(&descriptor).as_bytes(),
        )?;
        let (store, _) =
            EpisodeStore::open_native(Path::new(&selection.root), StoreOptions::default())?;
        EpisodeApplication::new(store, admission, DEFAULT_RESPONSE_LIMIT)
            .execute(&Request::parse(canonical_json(request).as_bytes())?)?;
        Ok(())
    }
}
fn progress_value(progress: InitialCompletionProgress) -> Value {
    let state = match &progress.state {
        InitialProgressState::Pending { stage, reason } => {
            json!({"status":"pending","stage":stage,"reason":reason})
        }
        InitialProgressState::OwnerReadsJoined => json!({"status":"owner_reads_joined"}),
        InitialProgressState::Conflicting { stage, reason } => {
            json!({"status":"conflicting","stage":stage,"reason":reason})
        }
        InitialProgressState::Unresolved {
            stage,
            owner_locator,
            reason,
        } => {
            json!({"status":"unresolved","stage":stage,"ownerLocator":owner_locator,"reason":reason})
        }
    };
    json!({"status":"initial_completion","progress":state,"locator":progress.locator,"lastCheckedRevision":progress.last_checked_revision,"stages":progress.stages,"newlyObservedReceipts":progress.newly_observed_receipts})
}
