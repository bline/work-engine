// Schema-4 retained remediation state. This file is included in application.rs
// so every transition uses the same campaign gate and operation receipts.

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemediationSessionRef {
    pub owner: String,
    pub root_id: String,
    pub initial_attempt_id: String,
    pub session_id: String,
    pub custody_reference: String,
    pub custody_sha256: String,
}

impl RemediationSessionRef {
    fn validate(&self) -> Result<()> {
        for value in [
            &self.owner,
            &self.root_id,
            &self.initial_attempt_id,
            &self.session_id,
            &self.custody_reference,
        ] {
            require_text(value, "retained session reference")?;
        }
        crate::require_sha(&self.custody_sha256, "retained custody")
    }
}

/// Trusted startup inputs. Their mere presence cannot attest actual custody;
/// provider entry also needs the provider-owned private session capability.
#[derive(Clone)]
pub struct RemediationSelectionParts {
    pub candidate_raw: Vec<u8>,
    pub profile_raw: Vec<u8>,
    pub gate_capture: Vec<u8>,
    pub gate_manifest: Vec<u8>,
    pub gate_receipt: Vec<u8>,
    pub selection: Value,
    pub completion: CompletionSelection,
    pub session: RemediationSessionRef,
    pub dispatch_command: RemediationDispatchCommand,
    pub acceptance_sources: Vec<Value>,
}

/// A request cannot deserialize an accepted successor selection.
/// ```compile_fail
/// let _: slice_campaign::RemediationSelection = serde_json::from_str("{}").unwrap();
/// ```
pub struct RemediationSelection {
    parts: RemediationSelectionParts,
}

impl RemediationSelection {
    pub fn new(parts: RemediationSelectionParts) -> Result<Self> {
        parts.session.validate()?;
        parts.completion.record.validate()?;
        if parts.acceptance_sources.is_empty() {
            return Err(CampaignError::Contract(
                "successor acceptance absent".into(),
            ));
        }
        for source in &parts.acceptance_sources {
            if source["status"] != "verified" {
                return Err(CampaignError::Contract(
                    "successor acceptance unverified".into(),
                ));
            }
            for key in ["owner", "reference", "revision", "freshness"] {
                text_field(source, key)?;
            }
            crate::require_sha(
                &text_field(source, "integrity_sha256")?,
                "acceptance source",
            )?;
        }
        for (name, raw) in [
            ("candidate", &parts.candidate_raw),
            ("profile", &parts.profile_raw),
            ("gate capture", &parts.gate_capture),
            ("gate manifest", &parts.gate_manifest),
            ("gate receipt", &parts.gate_receipt),
        ] {
            if raw.is_empty() || raw.len() > 1_048_576 || std::str::from_utf8(raw).is_err() {
                return Err(CampaignError::Contract(format!("{name} bytes invalid")));
            }
        }
        Ok(Self { parts })
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemediationRequestRef {
    pub root_id: String,
    pub identity: CampaignIdentity,
    pub obligation_id: String,
    pub round_id: String,
    pub generation: u64,
    pub operation_id: String,
    pub execution_attempt_id: String,
    pub prepared_revision: CampaignRevision,
    pub request_digest: String,
    pub candidate: crate::contract::CandidateRef,
    pub selection: crate::contract::SelectionRef,
    pub profile_digest: String,
    pub session: RemediationSessionRef,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemediationRound {
    pub request: RemediationRequestRef,
    pub initial_outcome: InitialOutcomeLocator,
    pub predecessor_generation: u64,
    pub predecessor_episode_revision: String,
    pub predecessor_result_canonical_json: String,
    pub predecessor_finding_refs: Vec<Value>,
    pub candidate_receipt_raw: String,
    pub physical_profile_raw: String,
    pub gate_capture_raw: String,
    pub gate_manifest_raw: String,
    pub gate_receipt_raw: String,
    pub accepted_selection: Value,
    pub accepted_claims: Vec<SelectedClaim>,
    pub frozen_dispatch_command: RemediationDispatchCommand,
    pub completion_selection: Value,
    pub acceptance_sources: Vec<Value>,
    pub dispatch: Option<RemediationDispatchRecord>,
    #[serde(default)]
    pub subject_read: Option<RemediationSubjectRead>,
    #[serde(default)]
    pub completion: Option<CompletionIntent>,
    pub outcome: Option<Value>,
    #[serde(default)]
    pub evaluations: Vec<RemediationEvaluationRecord>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemediationEvaluationRecord {
    pub operation_id: String,
    pub finding_id: String,
    pub finding_revision: String,
    pub valid: bool,
    pub consumer_tree: String,
    pub decision_scope: String,
    pub reliance: Value,
    pub projection: Value,
    pub publication_state: String,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemediationProgress {
    pub current_generation: u64,
    pub rounds: Vec<RemediationRound>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemediationPrepared {
    pub request: RemediationRequestRef,
    pub original_registration: RecoveryLocator,
}

#[derive(Debug)]
pub enum RemediationPreparation {
    Applied {
        prepared: RemediationPrepared,
        receipt: OperationReceipt,
    },
    Replayed {
        prepared: RemediationPrepared,
        receipt: OperationReceipt,
    },
    NoEffect(String),
    OutcomeUnknown(RecoveryLocator),
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemediationDispatchCommand {
    pub episode_id: String,
    pub writer_actor: String,
    pub runtime_session: String,
    pub reviewer_profile: String,
    pub execution_attempt_id: String,
    pub subject_transition_id_canonical_json: String,
    pub result_transition_id_canonical_json: String,
}

impl RemediationDispatchCommand {
    pub(crate) fn validate(&self, request: &RemediationRequestRef) -> Result<()> {
        for value in [
            &self.episode_id,
            &self.writer_actor,
            &self.runtime_session,
            &self.reviewer_profile,
            &self.execution_attempt_id,
        ] {
            require_text(value, "remediation dispatch")?;
        }
        if self.runtime_session != request.session.session_id
            || self.execution_attempt_id != request.execution_attempt_id
            || self.execution_attempt_id == request.identity.attempt_id
        {
            return Err(CampaignError::Contract(
                "remediation session or attempt differs".into(),
            ));
        }
        let expected_episode = crate::contract::episode_digest(&json!({
            "identity":request.identity,"obligationId":request.obligation_id,
        }))?;
        if self.episode_id != expected_episode[..32] {
            return Err(CampaignError::Contract(
                "remediation episode differs".into(),
            ));
        }
        let mut ids = Vec::new();
        for raw in [
            &self.subject_transition_id_canonical_json,
            &self.result_transition_id_canonical_json,
        ] {
            let value =
                parse_episode_json(raw).map_err(|e| CampaignError::Contract(e.to_string()))?;
            if review_episode_core::codec::canonical_json(&value) != *raw
                || value.nonempty_text("transition ID").is_err()
            {
                return Err(CampaignError::Contract(
                    "transition identity invalid".into(),
                ));
            }
            ids.push(value);
        }
        if ids[0] == ids[1] {
            return Err(CampaignError::Contract(
                "transition identities overlap".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemediationDispatchRecord {
    pub operation_id: String,
    pub request_digest: String,
    pub command: RemediationDispatchCommand,
    pub may_have_entered: bool,
}

/// Issued only by a new durable dispatch commit. Replays and recovery return
/// the saved record without a renewed provider-entry capability.
#[derive(Debug)]
pub struct RemediationDispatchPermit {
    owner_epoch: String,
    root_id: String,
    identity: CampaignIdentity,
    round_id: String,
    request_digest: String,
    operation_id: String,
    dispatched_revision: CampaignRevision,
}

#[derive(Debug)]
pub enum RemediationDispatchEffect {
    Applied {
        permit: RemediationDispatchPermit,
        receipt: OperationReceipt,
    },
    Replayed {
        record: RemediationDispatchRecord,
        receipt: OperationReceipt,
    },
    NoEffect(String),
    OutcomeUnknown(RecoveryLocator),
}

#[derive(Debug)]
pub struct DispatchedRemediation {
    pub request: RemediationRequestRef,
    pub command: RemediationDispatchCommand,
}

#[derive(Clone, Debug)]
pub struct RemediationRecovery {
    pub prepared: RemediationPrepared,
    pub dispatch: Option<RemediationDispatchRecord>,
    pub subject_read: Option<RemediationSubjectRead>,
    pub completion: Option<RemediationCompletionPrepared>,
    pub stages: Vec<StageIntent>,
    pub joined_readbacks: Option<JoinedReadbacks>,
    pub outcome: Option<Value>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemediationCompletionPrepared {
    pub round: RemediationRequestRef,
    pub completion_operation_id: String,
    pub original_registration: RecoveryLocator,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemediationBuilderEvaluationPrepared {
    pub round: RemediationRequestRef,
    pub evaluation_operation_id: String,
    pub original_registration: RecoveryLocator,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemediationSubjectRead {
    pub operation_id: String,
    pub transition_id_canonical_json: String,
    pub content_digest: String,
    pub resolved_revision: String,
    pub result_digest: String,
    pub authority_digest: String,
}

#[derive(Clone, Debug)]
pub enum RemediationStageOutcome {
    Ready {
        stage: Box<StageIntent>,
        progress: StageProgress,
    },
    Unresolved {
        owner_locator: Value,
        reason: String,
    },
    Conflicting {
        reason: String,
    },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RemediationFindingCommand {
    envelope: FindingEnvelope,
    predecessor_revision: Option<String>,
    initial_revision_ref: Value,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RemediationOutcomeManifest {
    operation_id: String,
    native_result_canonical_json: String,
    episode_canonical_json: String,
    finding_commands: Vec<RemediationFindingCommand>,
    selection: CompletionSelectionRecord,
    established: bool,
}

fn remediation_finding_commands(
    round: &RemediationRound,
    result: &EpisodeValue,
    episode: &EpisodeValue,
    operation_id: &str,
) -> Result<Vec<RemediationFindingCommand>> {
    let manifest = OutcomeManifest {
        joined: round.initial_outcome.joined.clone(),
        operation_id: operation_id.into(),
        selection: Some(decode(&round.completion_selection)?),
        native_result_canonical_json: review_episode_core::codec::canonical_json(result),
        episode_canonical_json: review_episode_core::codec::canonical_json(episode),
    };
    let envelopes = finding_envelopes(&manifest)?;
    let mut predecessor_by_native = std::collections::BTreeMap::new();
    for prior in &round.predecessor_finding_refs {
        let native_id = review_episode_core::codec::canonical_json(
            &parse_episode_json(&text_field(prior, "nativeIdCanonicalJson")?)
                .map_err(|e| CampaignError::Integrity(e.to_string()))?,
        );
        if predecessor_by_native.insert(native_id, prior).is_some() {
            return Err(CampaignError::Integrity(
                "duplicate predecessor finding".into(),
            ));
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut commands = Vec::with_capacity(envelopes.len());
    for envelope in envelopes {
        let native_id = review_episode_core::codec::canonical_json(
            &parse_episode_json(&envelope.native_id_json)
                .map_err(|e| CampaignError::Integrity(e.to_string()))?,
        );
        if !seen.insert(native_id.clone()) {
            return Err(CampaignError::Contract(
                "duplicate native finding ID".into(),
            ));
        }
        let subject = cp(&envelope.subject_json)?;
        let claim_id = ce(claim_evidence::stable_claim_id(&subject))?;
        let predecessor = predecessor_by_native.remove(&native_id);
        if predecessor.is_some_and(|prior| prior["claimId"] != claim_id.as_str()) {
            return Err(CampaignError::Conflict(
                "retained stable finding claim differs".into(),
            ));
        }
        let predecessor_revision = predecessor
            .map(|prior| text_field(prior, "revision"))
            .transpose()?;
        let initial_revision_ref =
            predecessor.map_or(Value::Null, |prior| prior["initialRevisionRef"].clone());
        commands.push(RemediationFindingCommand {
            envelope,
            predecessor_revision,
            initial_revision_ref,
        });
    }
    if !predecessor_by_native.is_empty() {
        return Err(CampaignError::Conflict(
            "retained result dropped a prior finding".into(),
        ));
    }
    Ok(commands)
}

fn remediation_finding_command(
    saved: &RemediationFindingCommand,
    admission: AdmissionBinding,
) -> Result<FindingCommand> {
    let subject = cp(&saved.envelope.subject_json)?;
    let mutation = if let Some(predecessor) = &saved.predecessor_revision {
        FindingMutation::Revise {
            claim_id: ce(claim_evidence::stable_claim_id(&subject))?,
            predecessor: ce(FindingRevisionId::new(predecessor.clone()))?,
            revision: cp(&saved.envelope.revision_json)?,
        }
    } else {
        FindingMutation::Create {
            subject,
            statement_identity: ce(cp(&saved.envelope.statement_json)?.as_text()).cloned()?,
            initial_revision: cp(&saved.envelope.revision_json)?,
        }
    };
    Ok(FindingCommand {
        operation_id: ce(OperationId::new(saved.envelope.operation_id.clone()))?,
        grant_id: saved.envelope.grant_id.clone(),
        admission,
        mutation,
    })
}

fn remediation_finding_record(
    receipt: &FindingReceipt,
    saved: &RemediationFindingCommand,
) -> Result<Value> {
    let reference = json!({"owner":"claim-evidence","reference":receipt.claim_id.as_str(),
        "revision":receipt.revision_id.as_str(),"sha256":receipt.revision_sha256,
        "freshness":"exact immutable revision"});
    let initial = if saved.predecessor_revision.is_some() {
        saved.initial_revision_ref.clone()
    } else {
        reference.clone()
    };
    let payload = cp(&saved.envelope.revision_json)?;
    Ok(json!({
        "claimId":receipt.claim_id.as_str(),"revision":receipt.revision_id.as_str(),
        "revisionSha256":receipt.revision_sha256,
        "locator":encode(&J2Locator::from(&receipt.locator))?,
        "nativeIdCanonicalJson":saved.envelope.native_id_json,
        "nativeIdDigest":saved.envelope.native_id_digest,
        "initialRevisionRef":initial,"revisionRef":reference,
        "predecessorRevision":saved.predecessor_revision,
        "outcomeCanonicalJson":cj(cf(cf(&payload,"profile_payload")?,"outcome")?)?,
    }))
}

fn remediation_finding_admission(
    state: &Snapshot,
    round: &RemediationRound,
    intent: &CompletionIntent,
    operation_id: &str,
) -> Result<AdmissionBinding> {
    let joined = intent
        .joined_readbacks
        .as_ref()
        .ok_or_else(|| CampaignError::Conflict("remediation readbacks pending".into()))?;
    Ok(AdmissionBinding {
        campaign_root_id: round.request.root_id.clone(),
        campaign_revision: state.revision.as_str().into(),
        campaign_operation_id: operation_id.into(),
        obligation_id: round.request.obligation_id.clone(),
        candidate_digest: round.request.candidate.receipt_sha256.clone(),
        selection_digest: round.request.selection.campaign_digest.clone(),
        profile_digest: round.request.profile_digest.clone(),
        prepared_request_sha256: round.request.request_digest.clone(),
        episode_revision: text_field(&joined.episode, "resolvedRevision")?,
        result_sha256: intent.native_result_claim_sha256.clone(),
        claims_request_sha256: String::new(),
    })
}

fn verify_remediation_published_findings(
    round: &RemediationRound,
    owners: &InitialCompletionOwners<'_>,
) -> Result<()> {
    let intent = round
        .completion
        .as_ref()
        .ok_or_else(|| CampaignError::Conflict("remediation completion absent".into()))?;
    for stage in intent
        .stages
        .iter()
        .filter(|s| s.kind == StageKind::Finding)
    {
        let saved: RemediationFindingCommand = decode(&stage.command)?;
        let locator = decode::<J2Locator>(&stage.owner_locator)?.owner()?;
        match owners.claims.claims.reconcile(&locator) {
            Reconciliation::Committed(receipt)
                if remediation_finding_record(&receipt, &saved)? == checked_ref(stage)? => {}
            Reconciliation::Absent => {
                return Err(CampaignError::Unsupported(
                    "committed remediation finding absent from CE".into(),
                ));
            }
            Reconciliation::Unresolved(reason) => return Err(CampaignError::Io(reason)),
            _ => {
                return Err(CampaignError::Conflict(
                    "committed remediation finding differs from CE".into(),
                ));
            }
        }
    }
    Ok(())
}

fn verify_remediation_reliances(
    round: &RemediationRound,
    owners: &InitialCompletionOwners<'_>,
) -> Result<Vec<claim_evidence::ExactRelianceRef>> {
    let intent = round
        .completion
        .as_ref()
        .ok_or_else(|| CampaignError::Conflict("remediation completion absent".into()))?;
    let mut reliances = Vec::new();
    for record in round
        .evaluations
        .iter()
        .filter(|r| r.publication_state == "committed")
    {
        let stage = intent
            .stages
            .iter()
            .find(|s| {
                s.kind == StageKind::EvaluationReliance
                    && s.stage_id == format!("{}:reliance", record.operation_id)
            })
            .ok_or_else(|| CampaignError::Integrity("evaluation reliance stage absent".into()))?;
        let J2Command::Evaluation(envelope) = decode(&stage.command)? else {
            return Err(CampaignError::Integrity(
                "evaluation command differs".into(),
            ));
        };
        let locator = decode::<J2Locator>(&stage.owner_locator)?.owner()?;
        let command = reliance_command(&envelope, binding_for(stage)?)?;
        if ce(owners.claims.claims.reliance_locator(&command))? != locator {
            return Err(CampaignError::Conflict(
                "committed reliance locator differs".into(),
            ));
        }
        let receipt = match owners.claims.claims.reconcile_reliance(&locator) {
            RelianceReconciliation::Committed(receipt) => receipt,
            RelianceReconciliation::Absent => {
                return Err(CampaignError::Unsupported(
                    "committed remediation reliance absent from CE".into(),
                ));
            }
            RelianceReconciliation::Conflicting => {
                return Err(CampaignError::Conflict(
                    "committed remediation reliance differs".into(),
                ));
            }
            RelianceReconciliation::Unresolved(reason) => return Err(CampaignError::Io(reason)),
        };
        let value = json!({"exact":value_from_claim(&receipt.exact.to_js_value())?,
            "locator":encode(&J2Locator::from(&receipt.locator))?});
        if record.reliance != value || checked_ref(stage)? != value {
            return Err(CampaignError::Conflict(
                "saved remediation reliance differs".into(),
            ));
        }
        reliances.push(receipt.exact);
    }
    Ok(reliances)
}

fn verify_remediation_finalized_outcome(
    round: &RemediationRound,
    saved: &Value,
    owners: &InitialCompletionOwners<'_>,
) -> Result<()> {
    verify_remediation_published_findings(round, owners)?;
    let reliances = verify_remediation_reliances(round, owners)?;
    let intent = round
        .completion
        .as_ref()
        .ok_or_else(|| CampaignError::Integrity("remediation completion absent".into()))?;
    let findings = saved["findings"]
        .as_array()
        .ok_or_else(|| CampaignError::Integrity("saved remediation findings absent".into()))?;
    if !established(intent)? {
        if !findings.is_empty() || !saved["projection"].is_null() || !reliances.is_empty() {
            return Err(CampaignError::Conflict(
                "unestablished remediation has published effects".into(),
            ));
        }
        return Ok(());
    }
    if findings.is_empty() {
        if !reliances.is_empty() {
            return Err(CampaignError::Conflict(
                "empty remediation has exact reliances".into(),
            ));
        }
        let joined = intent
            .joined_readbacks
            .as_ref()
            .ok_or_else(|| CampaignError::Integrity("remediation readbacks absent".into()))?;
        let body = json!({"schemaVersion":1,"kind":"no_findings",
            "remediationEpisodeRef":joined.episode,
            "generation":round.request.generation,
            "candidateTree":round.request.candidate.tree,
            "nativeResultClaimSha256":intent.native_result_claim_sha256,
            "findingSetSha256":ce(claim_digest(&ClaimValue::Array(vec![])))?,
            "count":0});
        let digest = ce(claim_digest(&claim_from_value(&body)?))?;
        let expected = json!({"kind":"no_findings","owner":"slice-campaign",
            "reference":format!("no-findings:{digest}"),"body":body});
        if saved["projection"] != expected {
            return Err(CampaignError::Conflict(
                "saved empty remediation projection differs".into(),
            ));
        }
        return Ok(());
    }
    let stage = stage_by_kind(intent, StageKind::FindingProjection)
        .ok_or_else(|| CampaignError::Integrity("finding projection stage absent".into()))?;
    let J2Command::Projection(envelope) = decode(&stage.command)? else {
        return Err(CampaignError::Integrity(
            "finding projection command differs".into(),
        ));
    };
    if checked_ref(stage)? != saved["projection"] {
        return Err(CampaignError::Conflict(
            "saved finding projection differs from registration".into(),
        ));
    }
    let original_json = text_field(&saved["projection"], "projectionCanonicalJson")?;
    let original = cp(&original_json)?;
    if cj(&original)? != original_json
        || ce(claim_digest(&original))? != text_field(&saved["projection"], "projectionSha256")?
    {
        return Err(CampaignError::Conflict(
            "saved finding projection digest differs".into(),
        ));
    }
    let projected = ce(owners
        .claims
        .claims
        .project_exact_revisions(&projection_command(&envelope, binding_for(stage)?)?))?;
    ce(owners.claims.claims.verify_projection_fresh(&projected))?;
    let original_revisions = cf(&original, "relevant_exact_revisions")?
        .as_array()
        .map_err(|e| CampaignError::Integrity(e.to_string()))?;
    let current_revisions = projected.relevant_exact_revisions();
    if original_revisions.len() != findings.len()
        || current_revisions.len() != findings.len()
        || original_revisions
            .iter()
            .zip(current_revisions)
            .any(|(original, current)| original != &current.revision)
        || findings
            .iter()
            .zip(current_revisions)
            .any(|(finding, current)| {
                finding["claimId"] != current.exact.claim_id.as_str()
                    || finding["revision"] != current.exact.revision_id.as_str()
                    || finding["revisionSha256"] != current.exact.revision_sha256
            })
    {
        return Err(CampaignError::Conflict(
            "saved finding projection no longer resolves exact CE revisions".into(),
        ));
    }
    // A later evaluation or generation can advance CE's store revision. The
    // saved projection is the immutable publication view; the fresh read above
    // checks its exact revisions without replacing that historical view.
    Ok(())
}

fn remediation_evaluation_view(
    round: &RemediationRound,
    record: &RemediationEvaluationRecord,
) -> Result<Value> {
    let mut outcome = round
        .outcome
        .clone()
        .ok_or_else(|| CampaignError::Integrity("remediation outcome absent".into()))?;
    let findings = outcome["findings"]
        .as_array()
        .ok_or_else(|| CampaignError::Integrity("remediation findings absent".into()))?;
    let result = parse_episode_json(
        outcome["nativeResultCanonicalJson"]
            .as_str()
            .ok_or_else(|| CampaignError::Integrity("native result absent".into()))?,
    )
    .map_err(|e| CampaignError::Integrity(e.to_string()))?;
    let native = result
        .get("findings")
        .ok_or_else(|| CampaignError::Integrity("native findings absent".into()))?
        .as_array()
        .map_err(|e| CampaignError::Integrity(e.to_string()))?;
    let all_done = !findings.is_empty()
        && findings.iter().all(|finding| {
            round.evaluations.iter().any(|evaluation| {
                evaluation.publication_state == "committed"
                    && finding["claimId"] == evaluation.finding_id
                    && finding["revision"] == evaluation.finding_revision
                    && (!evaluation.valid
                        || native.iter().any(|item| {
                            item.get("id").is_some_and(|id| {
                                finding["nativeIdCanonicalJson"]
                                    .as_str()
                                    .and_then(|raw| parse_episode_json(raw).ok())
                                    .is_some_and(|saved| saved == *id)
                            }) && item.get("status")
                                == Some(&EpisodeValue::text("verified_resolved"))
                        }))
            })
        });
    if all_done {
        outcome["kind"] = json!("reported");
    }
    Ok(json!({
        "operationId":record.operation_id,
        "claimId":record.finding_id,
        "findingRevision":record.finding_revision,
        "valid":record.valid,
        "consumerTree":record.consumer_tree,
        "decisionScope":record.decision_scope,
        "reliance":record.reliance,
        "projection":record.projection,
        "outcome":outcome,
        "evaluations":round.evaluations,
    }))
}

fn candidate_episode_reference(
    authority: &review_episode_core::identity::Authority,
    candidate: &crate::contract::CandidateRef,
) -> Result<EpisodeValue> {
    let subject = EpisodeValue::object([
        ("commit", EpisodeValue::text(&candidate.commit)),
        ("tree", EpisodeValue::text(&candidate.tree)),
        (
            "patchIdentity",
            EpisodeValue::text(&candidate.patch_identity),
        ),
    ]);
    let mut reference = authority
        .initial_subject()
        .as_object()
        .map_err(|e| CampaignError::Contract(e.to_string()))?
        .clone();
    reference.insert(
        EpisodeString::new("revision"),
        EpisodeValue::text(&candidate.commit),
    );
    reference.insert(
        EpisodeString::new("sha256"),
        EpisodeValue::text(&episode_value_digest(&subject)),
    );
    Ok(EpisodeValue::Object(reference))
}

fn remediation_round<'a>(state: &'a Snapshot, round_id: &str) -> Result<&'a RemediationRound> {
    state
        .remediation
        .as_ref()
        .and_then(|p| p.rounds.iter().find(|r| r.request.round_id == round_id))
        .ok_or_else(|| CampaignError::Conflict("remediation round absent".into()))
}
fn remediation_round_mut<'a>(
    state: &'a mut Snapshot,
    round_id: &str,
) -> Result<&'a mut RemediationRound> {
    state
        .remediation
        .as_mut()
        .and_then(|p| p.rounds.iter_mut().find(|r| r.request.round_id == round_id))
        .ok_or_else(|| CampaignError::Conflict("remediation round absent".into()))
}

fn remediation_claim(round: &RemediationRound, boundary: &str) -> Result<ClaimValue> {
    let claims = round.accepted_selection["specialists"]
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item["obligationId"] == round.request.obligation_id)
        })
        .and_then(|item| item["requiredClaims"].as_array())
        .ok_or_else(|| CampaignError::Integrity("remediation selected pair absent".into()))?;
    let claim = claims
        .iter()
        .find(|claim| claim["consumptionBoundary"] == boundary)
        .ok_or_else(|| CampaignError::Integrity("remediation boundary claim absent".into()))?;
    claim_from_value(claim)
}

fn remediation_ce_request(
    state: &Snapshot,
    round: &RemediationRound,
    intent: &CompletionIntent,
    kind: StageKind,
    checked: &CheckedExecutionResult,
    grant_id: &str,
) -> Result<ProductionPathRequest> {
    let stage = match kind {
        StageKind::Observation => ProductionPathStage::RecordObservation,
        StageKind::EstablishmentBuilder | StageKind::EstablishmentCampaign => {
            ProductionPathStage::EstablishClaim
        }
        StageKind::AdmissionBuilder | StageKind::AdmissionCampaign => {
            ProductionPathStage::ReadAdmission
        }
        _ => return Err(CampaignError::Contract("not a remediation CE stage".into())),
    };
    let id = stage_id(&intent.campaign_operation_id, kind);
    let admission = ProductionPathAdmissionBinding {
        campaign_root_id: round.request.root_id.clone(),
        campaign_revision: state.revision.as_str().into(),
        campaign_operation_id: intent.campaign_operation_id.clone(),
        obligation_id: round.request.obligation_id.clone(),
        candidate_digest: round.request.candidate.receipt_sha256.clone(),
        selection_digest: round.request.selection.campaign_digest.clone(),
        profile_digest: round.request.profile_digest.clone(),
        prepared_request_sha256: round.request.request_digest.clone(),
        child_request_sha256: String::new(),
        review_episode_id: round.frozen_dispatch_command.episode_id.clone(),
        attempt_id: round.request.execution_attempt_id.clone(),
        native_result_sha256: intent.native_result_claim_sha256.clone(),
        episode_revision: None,
        session_id: match intent.observation_selection {
            ObservationSelection::Present { .. } => intent.execution_session_id.clone(),
            ObservationSelection::Absent { .. } => None,
        },
        stage,
        access: ProductionPathAccess::Original,
    };
    let mut request = match kind {
        StageKind::Observation => {
            let CheckedObservation::Present(observation) = checked.observation() else {
                return Err(CampaignError::Conflict(
                    "remediation observation absent".into(),
                ));
            };
            ProductionPathRequest::RecordObservation(ObservationCommand {
                grant_id: grant_id.into(),
                admission,
                observation: observation.clone(),
            })
        }
        StageKind::EstablishmentBuilder | StageKind::EstablishmentCampaign => {
            let boundary = if kind == StageKind::EstablishmentBuilder {
                "builder_projection"
            } else {
                "campaign_terminalization"
            };
            ProductionPathRequest::EstablishClaim(EstablishmentCommand {
                grant_id: grant_id.into(),
                admission,
                operation_id: id,
                claim: remediation_claim(round, boundary)?,
                observation_id: match &intent.observation_selection {
                    ObservationSelection::Present { observation_id, .. } => {
                        Some(observation_id.clone())
                    }
                    ObservationSelection::Absent { .. } => None,
                },
            })
        }
        StageKind::AdmissionBuilder | StageKind::AdmissionCampaign => {
            let builder = kind == StageKind::AdmissionBuilder;
            let predecessor = if builder {
                StageKind::EstablishmentBuilder
            } else {
                StageKind::EstablishmentCampaign
            };
            let receipt = committed_receipt(intent, predecessor)?.ok_or_else(|| {
                CampaignError::Conflict("remediation establishment pending".into())
            })?;
            let boundary = if builder {
                "builder_projection"
            } else {
                "campaign_terminalization"
            };
            let (observation_id, observation_sha256) = match &intent.observation_selection {
                ObservationSelection::Present {
                    observation_id,
                    observation_sha256,
                    ..
                } => (
                    Some(observation_id.clone()),
                    Some(observation_sha256.clone()),
                ),
                ObservationSelection::Absent { .. } => (None, None),
            };
            ProductionPathRequest::ReadAdmission(AdmissionReadRequest {
                grant_id: grant_id.into(),
                admission,
                claim: remediation_claim(round, boundary)?,
                establishment_id: receipt.id,
                establishment_sha256: receipt.sha256,
                establishment_request_sha256: receipt.locator.payload_sha256,
                establishment_admission_sha256: receipt.locator.admission_sha256,
                observation_id,
                observation_sha256,
            })
        }
        _ => unreachable!(),
    };
    set_request_hash(&mut request)?;
    Ok(request)
}

fn remediation_episode_request(
    round: &RemediationRound,
    stage: &StageIntent,
) -> Result<review_episode::in_process_admission::EpisodeReadRequest> {
    let transition = parse_episode_json(
        &round
            .frozen_dispatch_command
            .result_transition_id_canonical_json,
    )
    .map_err(|e| CampaignError::Integrity(e.to_string()))?
    .as_text()
    .map_err(|e| CampaignError::Integrity(e.to_string()))?
    .clone();
    let identity = review_episode_core::identity::Identity::parse(
        &parse_episode_json(
            &json!({
                "runId":round.request.identity.run_id,
                "sliceNumber":round.request.identity.slice_number,
                "attemptId":round.request.identity.attempt_id,
                "planVersion":round.request.identity.plan_version,
                "reviewObligationId":round.request.obligation_id,
                "reviewEpisodeId":round.frozen_dispatch_command.episode_id,
            })
            .to_string(),
        )
        .map_err(|e| CampaignError::Integrity(e.to_string()))?,
    )
    .map_err(|e| CampaignError::Integrity(e.to_string()))?;
    Ok(
        review_episode::in_process_admission::EpisodeReadRequest::Transition {
            identity,
            transition_id: transition,
            content_digest: EpisodeRevision(stage.request_sha256.clone()),
        },
    )
}

impl CampaignInner {
    fn check_remediation_finding_admission(
        &self,
        state: &Snapshot,
        binding: &AdmissionBinding,
    ) -> Result<()> {
        let progress = state
            .remediation
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("remediation round absent".into()))?;
        let round = progress
            .rounds
            .iter()
            .find(|round| round.request.request_digest == binding.prepared_request_sha256)
            .ok_or_else(|| CampaignError::Conflict("remediation attempt absent".into()))?;
        let intent = round
            .completion
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("remediation completion absent".into()))?;
        let joined = intent
            .joined_readbacks
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("remediation readbacks pending".into()))?;
        let installed = self
            .initial_join
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("remediation owner absent".into()))?;
        let selected: CompletionSelectionRecord = decode(&round.completion_selection)?;
        selected.validate()?;
        if round.request.generation != progress.current_generation
            || joined.builder["status"] != "established"
            || joined.campaign["status"] != "established"
            || binding.campaign_root_id != self.store.root_id
            || binding.obligation_id != round.request.obligation_id
            || binding.candidate_digest != round.request.candidate.receipt_sha256
            || binding.selection_digest != round.request.selection.campaign_digest
            || binding.profile_digest != round.request.profile_digest
            || binding.prepared_request_sha256 != round.request.request_digest
            || binding.episode_revision != joined.episode["resolvedRevision"]
            || binding.result_sha256 != intent.native_result_claim_sha256
            || selected.consumer_tree != round.request.candidate.tree
            || selected.root.owner() != installed.selection.claims_root
            || installed.digest != intent.owner_selection_digest
            || installed.epoch != self.store.epoch
            || self
                .store
                .dispatch_slot(&state.identity, &round.request.obligation_id)?
                != round
                    .dispatch
                    .as_ref()
                    .map(|record| (Some(record.operation_id.clone()), true))
        {
            return Err(CampaignError::Conflict(
                "remediation finding authority differs".into(),
            ));
        }
        let stage = intent
            .stages
            .iter()
            .find(|stage| {
                matches!(
                    stage.kind,
                    StageKind::Finding
                        | StageKind::FindingProjection
                        | StageKind::EvaluationReliance
                        | StageKind::EvaluationProjection
                ) && stage.original_admission_sha256 == binding.digest()
                    && stage.request_sha256 == binding.claims_request_sha256
            })
            .ok_or_else(|| {
                CampaignError::Conflict(
                    "exact remediation finding request is not registered".into(),
                )
            })?;
        if stage.original_admission != encode(&J2Admission::from(binding))?
            || stage.registration_base_revision.as_str() != binding.campaign_revision
            || stage.evidence_ref != intent.execution_evidence_ref
            || stage.owner_selection_digest != intent.owner_selection_digest
        {
            return Err(CampaignError::Conflict(
                "remediation finding admission differs".into(),
            ));
        }
        let receipt = self
            .store
            .receipt(&stage.registration_operation_id)?
            .ok_or_else(|| CampaignError::Integrity("finding registration absent".into()))?;
        let kind = if stage.kind == StageKind::Finding {
            "register_remediation_finding"
        } else if stage.kind == StageKind::FindingProjection {
            "register_remediation_projection"
        } else {
            "prepare_remediation_evaluation"
        };
        if receipt.kind != kind
            || receipt.prior_revision.as_ref() != Some(&stage.registration_base_revision)
            || receipt.identity != state.identity
            || receipt.root_id != self.store.root_id
        {
            return Err(CampaignError::Conflict(
                "remediation finding registration differs".into(),
            ));
        }
        Ok(())
    }
}

impl Campaign {
    pub fn prepare_remediation_builder_evaluation(
        &mut self,
        completion: &RemediationCompletionPrepared,
        expected: &CampaignRevision,
        operation_id: &str,
        decision: BuilderEvaluationDecision,
        owners: &mut InitialCompletionOwners<'_>,
    ) -> Result<Effect<RemediationBuilderEvaluationPrepared>> {
        require_text(operation_id, "remediation evaluation operation")?;
        checked_owner_wrapper(self, owners)?;
        let mut gate = self.owner()?;
        if let Some((receipt, saved)) = gate.store.replay_result(operation_id)? {
            let round = remediation_round(&saved, &completion.round.round_id)?;
            let record = round
                .evaluations
                .iter()
                .find(|r| r.operation_id == operation_id)
                .ok_or_else(|| CampaignError::Integrity("evaluation replay absent".into()))?;
            if receipt.kind != "prepare_remediation_evaluation"
                || receipt.prior_revision.as_ref() != Some(expected)
                || record.finding_id != decision.claim_id.as_str()
                || record.finding_revision != decision.finding_revision.as_str()
                || record.valid != decision.valid
            {
                return Err(CampaignError::Conflict(
                    "remediation evaluation replay differs".into(),
                ));
            }
            return Ok(Effect::Replayed {
                value: RemediationBuilderEvaluationPrepared {
                    round: round.request.clone(),
                    evaluation_operation_id: operation_id.into(),
                    original_registration: gate.locator(
                        &saved.identity,
                        operation_id,
                        "prepare_remediation_evaluation",
                        receipt.request_digest.clone(),
                        receipt.prior_revision.clone(),
                    ),
                },
                receipt,
            });
        }
        let state = gate.current(&completion.round.identity, expected)?;
        let round = remediation_round(&state, &completion.round.round_id)?;
        let intent = round
            .completion
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("remediation completion absent".into()))?;
        let outcome = round
            .outcome
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("remediation outcome pending".into()))?;
        if round.request.generation != state.remediation.as_ref().unwrap().current_generation
            || round.request.request_digest != completion.round.request_digest
            || intent.campaign_operation_id != completion.completion_operation_id
            || !matches!(
                outcome["kind"].as_str(),
                Some("reported" | "awaiting_builder")
            )
        {
            return Err(CampaignError::Conflict(
                "usable current remediation outcome absent".into(),
            ));
        }
        let findings = outcome["findings"]
            .as_array()
            .ok_or_else(|| CampaignError::Integrity("remediation finding set absent".into()))?;
        let finding = findings
            .iter()
            .find(|finding| {
                finding["claimId"] == decision.claim_id.as_str()
                    && finding["revision"] == decision.finding_revision.as_str()
            })
            .ok_or_else(|| CampaignError::Conflict("evaluation is not a current finding".into()))?;
        if round.evaluations.iter().any(|r| {
            r.finding_id == decision.claim_id.as_str()
                && r.finding_revision == decision.finding_revision.as_str()
        }) {
            return Err(CampaignError::Conflict(
                "finding already has a disposition".into(),
            ));
        }
        let selection: CompletionSelectionRecord = decode(&round.completion_selection)?;
        selection.validate()?;
        let envelope = EvaluationEnvelope {
            operation_id: operation_id.into(),
            grant_id: selection.reliance_grant_id.clone(),
            claim_id: decision.claim_id.as_str().into(),
            finding_revision: decision.finding_revision.as_str().into(),
            valid: decision.valid,
            consumer: selection.consumer.clone(),
            consumer_tree: selection.consumer_tree.clone(),
            decision_scope: selection.decision_scope.clone(),
        };
        let mut reliance = reliance_command(
            &envelope,
            remediation_finding_admission(&state, round, intent, operation_id)?,
        )?;
        reliance.admission.claims_request_sha256 =
            ce(claim_evidence::reliance_request_sha256(&reliance))?;
        let locator = ce(owners.claims.claims.reliance_locator(&reliance))?;
        if locator.root != selection.root.owner()
            || locator.grant_sha256 != selection.reliance_grant_sha256
        {
            return Err(CampaignError::Conflict(
                "remediation reliance grant differs".into(),
            ));
        }
        let stage = make_stage(
            intent,
            &state,
            format!("{operation_id}:reliance"),
            StageKind::EvaluationReliance,
            operation_id,
            J2Command::Evaluation(envelope),
            Some(reliance.admission),
            encode(&J2Locator::from(&locator))?,
        )?;
        let projection_envelope = ProjectionEnvelope {
            request_id: format!("{operation_id}:projection"),
            consumer: selection.consumer.clone(),
            consumer_tree: selection.consumer_tree.clone(),
            decision_scope: selection.decision_scope.clone(),
            revisions: findings
                .iter()
                .map(|f| text_field(f, "revision"))
                .collect::<Result<Vec<_>>>()?,
        };
        let mut projection = projection_command(
            &projection_envelope,
            remediation_finding_admission(&state, round, intent, operation_id)?,
        )?;
        projection.admission.claims_request_sha256 =
            ce(claim_evidence::projection_request_sha256(&projection))?;
        let projection_stage = make_stage(
            intent,
            &state,
            projection_envelope.request_id.clone(),
            StageKind::EvaluationProjection,
            operation_id,
            J2Command::Projection(projection_envelope),
            Some(projection.admission),
            encode(&selection.root)?,
        )?;
        let record = RemediationEvaluationRecord {
            operation_id: operation_id.into(),
            finding_id: text_field(finding, "claimId")?,
            finding_revision: text_field(finding, "revision")?,
            valid: decision.valid,
            consumer_tree: selection.consumer_tree,
            decision_scope: selection.decision_scope,
            reliance: Value::Null,
            projection: Value::Null,
            publication_state: "registered".into(),
        };
        let digest = campaign_digest(&json!({"completion":completion,
            "expectedRevision":expected,"record":record,
            "stage":stage,"projectionStage":projection_stage}))?;
        let mut next = state.clone();
        let target = remediation_round_mut(&mut next, &completion.round.round_id)?;
        target
            .completion
            .as_mut()
            .unwrap()
            .stages
            .extend([stage, projection_stage]);
        target.evaluations.push(record);
        next.revision = state_revision(&next)?;
        if serde_json::to_vec(&next)
            .map_err(|e| CampaignError::Contract(e.to_string()))?
            .len()
            > 8_388_608
        {
            return Err(CampaignError::Contract(
                "remediation evaluation exceeds capacity".into(),
            ));
        }
        let locator = gate.locator(
            &state.identity,
            operation_id,
            "prepare_remediation_evaluation",
            digest.clone(),
            Some(expected.clone()),
        );
        let receipt = gate.receipt(
            &next,
            Some(expected.clone()),
            operation_id,
            "prepare_remediation_evaluation",
            digest,
        );
        let value = RemediationBuilderEvaluationPrepared {
            round: round.request.clone(),
            evaluation_operation_id: operation_id.into(),
            original_registration: locator.clone(),
        };
        Ok(
            match gate
                .store
                .write(&next, Some(expected), &receipt, false, None)
            {
                Ok(()) => Effect::Applied { value, receipt },
                Err(CampaignError::Conflict(e) | CampaignError::Contract(e)) => Effect::NoEffect(e),
                Err(_) => Effect::OutcomeUnknown(locator),
            },
        )
    }

    pub fn complete_remediation_builder_evaluation(
        &mut self,
        prepared: &RemediationBuilderEvaluationPrepared,
        episode: &mut EpisodeApplication<InProcessReadAdmission>,
        owners: &mut InitialCompletionOwners<'_>,
    ) -> Result<Effect<Value>> {
        let (round, intent, record, completion) = {
            let gate = self.owner()?;
            let state = gate
                .store
                .read(&prepared.round.identity)?
                .ok_or_else(|| CampaignError::Integrity("remediation campaign absent".into()))?;
            let round = remediation_round(&state, &prepared.round.round_id)?.clone();
            let intent = round
                .completion
                .as_ref()
                .ok_or_else(|| CampaignError::Conflict("remediation completion absent".into()))?
                .clone();
            let record = round
                .evaluations
                .iter()
                .find(|record| record.operation_id == prepared.evaluation_operation_id)
                .ok_or_else(|| CampaignError::Conflict("remediation evaluation absent".into()))?
                .clone();
            let receipt = gate.store.receipt(&record.operation_id)?.ok_or_else(|| {
                CampaignError::Integrity("remediation evaluation registration absent".into())
            })?;
            if round.request.request_digest != prepared.round.request_digest
                || receipt.kind != "prepare_remediation_evaluation"
                || prepared.original_registration.request_digest != receipt.request_digest
            {
                return Err(CampaignError::Conflict("evaluation locator differs".into()));
            }
            let completion_receipt = gate
                .store
                .receipt(&intent.campaign_operation_id)?
                .ok_or_else(|| CampaignError::Integrity("completion receipt absent".into()))?;
            let completion = RemediationCompletionPrepared {
                round: round.request.clone(),
                completion_operation_id: intent.campaign_operation_id.clone(),
                original_registration: gate.locator(
                    &state.identity,
                    &intent.campaign_operation_id,
                    "prepare_remediation_completion",
                    completion_receipt.request_digest,
                    Some(intent.registration_base_revision.clone()),
                ),
            };
            (round, intent, record, completion)
        };
        match self.complete_remediation(&completion, episode, owners)? {
            Effect::Applied { .. } | Effect::Replayed { .. } => {}
            Effect::NoEffect(reason) => return Ok(Effect::NoEffect(reason)),
            Effect::OutcomeUnknown(locator) => return Ok(Effect::OutcomeUnknown(locator)),
        }
        verify_remediation_published_findings(&round, owners)?;
        let old_reliances = verify_remediation_reliances(&round, owners)?;
        if record.publication_state == "committed" {
            let stage = intent
                .stages
                .iter()
                .find(|s| {
                    s.kind == StageKind::EvaluationProjection
                        && s.stage_id == format!("{}:projection", record.operation_id)
                })
                .ok_or_else(|| CampaignError::Integrity("evaluation projection absent".into()))?;
            let J2Command::Projection(envelope) = decode(&stage.command)? else {
                return Err(CampaignError::Integrity(
                    "evaluation projection differs".into(),
                ));
            };
            let projected = ce(owners
                .claims
                .claims
                .project_exact_revisions(&projection_command(&envelope, binding_for(stage)?)?))?;
            for reliance in &old_reliances {
                if !projected.relevant_exact_revisions().iter().any(|revision| {
                    revision.exact == reliance.finding
                        && revision.current_reliances.contains(reliance)
                }) {
                    return Err(CampaignError::Conflict(
                        "aggregate lacks older exact reliance".into(),
                    ));
                }
            }
            ce(owners.claims.claims.verify_projection_fresh(&projected))?;
            if projection_value(&projected)? != record.projection {
                return Err(CampaignError::Conflict(
                    "saved evaluation projection differs".into(),
                ));
            }
            let view = remediation_evaluation_view(&round, &record)?;
            let receipt = self
                .owner()?
                .store
                .receipt(&format!("{}:publish", record.operation_id))?
                .ok_or_else(|| CampaignError::Integrity("evaluation publication absent".into()))?;
            return Ok(Effect::Replayed {
                value: view,
                receipt,
            });
        }
        let stage = intent
            .stages
            .iter()
            .find(|s| {
                s.kind == StageKind::EvaluationReliance
                    && s.stage_id == format!("{}:reliance", record.operation_id)
            })
            .ok_or_else(|| CampaignError::Integrity("reliance stage absent".into()))?;
        let J2Command::Evaluation(envelope) = decode(&stage.command)? else {
            return Err(CampaignError::Integrity("reliance command differs".into()));
        };
        let locator = decode::<J2Locator>(&stage.owner_locator)?.owner()?;
        let command = reliance_command(&envelope, binding_for(stage)?)?;
        if ce(owners.claims.claims.reliance_locator(&command))? != locator {
            return Err(CampaignError::Conflict(
                "remediation reliance locator differs".into(),
            ));
        }
        let receipt = match owners.claims.claims.reconcile_reliance(&locator) {
            RelianceReconciliation::Committed(value) => *value,
            RelianceReconciliation::Absent => match owners.claims.claims.record_reliance(command) {
                Ok(ReliancePublication::Applied(value) | ReliancePublication::Replayed(value)) => {
                    value
                }
                Err(claim_evidence::WriteError::NoEffect(error)) => {
                    return Err(CampaignError::Conflict(error.to_string()));
                }
                Err(claim_evidence::WriteError::OutcomeUnknown(locator)) => {
                    return Err(CampaignError::Io(format!(
                        "reliance uncertain: {locator:?}"
                    )));
                }
            },
            RelianceReconciliation::Conflicting => {
                return Err(CampaignError::Conflict(
                    "remediation reliance owner conflicts".into(),
                ));
            }
            RelianceReconciliation::Unresolved(reason) => return Err(CampaignError::Io(reason)),
        };
        let reliance = json!({"exact":value_from_claim(&receipt.exact.to_js_value())?,
            "locator":encode(&J2Locator::from(&receipt.locator))?});
        match self.record_remediation_stage_progress(
            &completion,
            StageKind::EvaluationReliance,
            Some(&stage.stage_id),
            checked_progress(reliance.clone())?,
        )? {
            Effect::Applied { .. } | Effect::Replayed { .. } | Effect::NoEffect(_) => {}
            Effect::OutcomeUnknown(locator) => return Ok(Effect::OutcomeUnknown(locator)),
        }
        let state = self
            .read(&prepared.round.identity)?
            .ok_or_else(|| CampaignError::Integrity("remediation campaign absent".into()))?;
        let round = remediation_round(&state, &prepared.round.round_id)?;
        let intent = round.completion.as_ref().unwrap();
        let projection_stage = intent
            .stages
            .iter()
            .find(|s| {
                s.kind == StageKind::EvaluationProjection
                    && s.stage_id == format!("{}:projection", record.operation_id)
            })
            .ok_or_else(|| CampaignError::Integrity("evaluation projection stage absent".into()))?;
        let J2Command::Projection(envelope) = decode(&projection_stage.command)? else {
            return Err(CampaignError::Integrity(
                "evaluation projection command differs".into(),
            ));
        };
        let projected = ce(owners
            .claims
            .claims
            .project_exact_revisions(&projection_command(
                &envelope,
                binding_for(projection_stage)?,
            )?))?;
        let mut all_reliances = old_reliances;
        all_reliances.push(receipt.exact.clone());
        for reliance in &all_reliances {
            if !projected.relevant_exact_revisions().iter().any(|revision| {
                revision.exact == reliance.finding && revision.current_reliances.contains(reliance)
            }) {
                return Err(CampaignError::Conflict(
                    "fresh aggregate lacks exact reliance".into(),
                ));
            }
        }
        ce(owners.claims.claims.verify_projection_fresh(&projected))?;
        let projection = projection_value(&projected)?;
        match self.record_remediation_stage_progress(
            &completion,
            StageKind::EvaluationProjection,
            Some(&projection_stage.stage_id),
            checked_progress(projection.clone())?,
        )? {
            Effect::Applied { .. } | Effect::Replayed { .. } | Effect::NoEffect(_) => {}
            Effect::OutcomeUnknown(locator) => return Ok(Effect::OutcomeUnknown(locator)),
        }
        let mut gate = self.owner()?;
        let state = gate
            .store
            .read(&prepared.round.identity)?
            .ok_or_else(|| CampaignError::Integrity("remediation campaign absent".into()))?;
        let mut next = state.clone();
        let round = remediation_round_mut(&mut next, &prepared.round.round_id)?;
        let evaluation = round
            .evaluations
            .iter_mut()
            .find(|r| r.operation_id == record.operation_id)
            .ok_or_else(|| CampaignError::Integrity("evaluation vanished".into()))?;
        if evaluation.publication_state != "registered" {
            return Err(CampaignError::Conflict(
                "evaluation publication moved".into(),
            ));
        }
        evaluation.reliance = reliance;
        evaluation.projection = projection;
        evaluation.publication_state = "committed".into();
        let committed = evaluation.clone();
        let view = remediation_evaluation_view(round, &committed)?;
        next.revision = state_revision(&next)?;
        let digest = campaign_digest(&view)?;
        let operation_id = format!("{}:publish", record.operation_id);
        let locator = gate.locator(
            &state.identity,
            &operation_id,
            "publish_remediation_evaluation",
            digest.clone(),
            Some(state.revision.clone()),
        );
        let receipt = gate.receipt(
            &next,
            Some(state.revision.clone()),
            &operation_id,
            "publish_remediation_evaluation",
            digest,
        );
        Ok(
            match gate
                .store
                .write(&next, Some(&state.revision), &receipt, false, None)
            {
                Ok(()) => Effect::Applied {
                    value: view,
                    receipt,
                },
                Err(CampaignError::Conflict(e) | CampaignError::Contract(e)) => Effect::NoEffect(e),
                Err(_) => Effect::OutcomeUnknown(locator),
            },
        )
    }
    fn register_remediation_outcome(
        &mut self,
        completion: &RemediationCompletionPrepared,
        manifest: &RemediationOutcomeManifest,
        owners: &InitialCompletionOwners<'_>,
    ) -> Result<StageIntent> {
        let mut gate = self.owner()?;
        let state = gate
            .store
            .read(&completion.round.identity)?
            .ok_or_else(|| CampaignError::Integrity("remediation campaign absent".into()))?;
        let round = remediation_round(&state, &completion.round.round_id)?;
        let intent = round
            .completion
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("remediation completion absent".into()))?;
        if round.request.generation != state.remediation.as_ref().unwrap().current_generation
            || intent.campaign_operation_id != completion.completion_operation_id
            || intent.joined_readbacks.is_none()
        {
            return Err(CampaignError::Conflict(
                "remediation outcome registration closed".into(),
            ));
        }
        let selection: CompletionSelectionRecord = decode(&round.completion_selection)?;
        if selection != manifest.selection {
            return Err(CampaignError::Conflict(
                "remediation outcome authority differs".into(),
            ));
        }
        if let Some(stage) = stage_by_kind(intent, StageKind::InitialOutcome) {
            if stage.command != encode(manifest)?
                || stage.registration_operation_id != manifest.operation_id
            {
                return Err(CampaignError::Conflict(
                    "remediation outcome registration differs".into(),
                ));
            }
            return Ok(stage.clone());
        }
        if round.outcome.is_some() {
            return Err(CampaignError::Conflict(
                "remediation outcome already published".into(),
            ));
        }
        if manifest.finding_commands.len() > 100 {
            return Err(CampaignError::Contract(
                "remediation finding count exceeds 100".into(),
            ));
        }
        if manifest.established {
            for saved in &manifest.finding_commands {
                let mut command = remediation_finding_command(
                    saved,
                    remediation_finding_admission(
                        &state,
                        round,
                        intent,
                        &saved.envelope.operation_id,
                    )?,
                )?;
                command.admission.claims_request_sha256 =
                    ce(claim_evidence::claims_request_sha256(&command))?;
                let locator = ce(owners.claims.claims.locator(&command))?;
                if locator.grant_sha256 != selection.finding_grant_sha256
                    || locator.root != selection.root.owner()
                {
                    return Err(CampaignError::Conflict(
                        "finding preflight grant differs".into(),
                    ));
                }
            }
        }
        let command = encode(manifest)?;
        let command_digest = campaign_digest(&command)?;
        let stage = StageIntent {
            stage_id: format!("{}:outcome", manifest.operation_id),
            kind: StageKind::InitialOutcome,
            registration_operation_id: manifest.operation_id.clone(),
            registration_base_revision: state.revision.clone(),
            command,
            request_sha256: command_digest.clone(),
            original_admission: Value::Null,
            original_admission_sha256: command_digest,
            owner_selection_digest: intent.owner_selection_digest.clone(),
            owner_locator: json!({"operationId":manifest.operation_id}),
            evidence_ref: intent.execution_evidence_ref.clone(),
            progress: StageProgress::Registered,
        };
        let digest = campaign_digest(&json!({"completion":completion,"stage":stage}))?;
        let mut next = state.clone();
        remediation_round_mut(&mut next, &completion.round.round_id)?
            .completion
            .as_mut()
            .unwrap()
            .stages
            .push(stage.clone());
        next.revision = state_revision(&next)?;
        if serde_json::to_vec(&next)
            .map_err(|e| CampaignError::Contract(e.to_string()))?
            .len()
            > 8_388_608
        {
            return Err(CampaignError::Contract(
                "remediation outcome preflight exceeds capacity".into(),
            ));
        }
        let receipt = gate.receipt(
            &next,
            Some(state.revision.clone()),
            &manifest.operation_id,
            "register_remediation_outcome",
            digest,
        );
        gate.store
            .write(&next, Some(&state.revision), &receipt, false, None)?;
        Ok(stage)
    }

    fn register_remediation_finding(
        &mut self,
        completion: &RemediationCompletionPrepared,
        saved: &RemediationFindingCommand,
        owners: &InitialCompletionOwners<'_>,
    ) -> Result<StageIntent> {
        let mut gate = self.owner()?;
        let state = gate
            .store
            .read(&completion.round.identity)?
            .ok_or_else(|| CampaignError::Integrity("remediation campaign absent".into()))?;
        let round = remediation_round(&state, &completion.round.round_id)?;
        let intent = round
            .completion
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("remediation completion absent".into()))?;
        if round.request.generation != state.remediation.as_ref().unwrap().current_generation
            || intent.campaign_operation_id != completion.completion_operation_id
            || intent.joined_readbacks.is_none()
            || round.outcome.is_some()
        {
            return Err(CampaignError::Conflict(
                "remediation finding registration closed".into(),
            ));
        }
        let id = &saved.envelope.operation_id;
        if let Some(stage) = intent.stages.iter().find(|stage| stage.stage_id == *id) {
            if stage.kind != StageKind::Finding || stage.command != encode(saved)? {
                return Err(CampaignError::Conflict(
                    "registered remediation finding differs".into(),
                ));
            }
            return Ok(stage.clone());
        }
        let selection: CompletionSelectionRecord = decode(&round.completion_selection)?;
        selection.validate()?;
        let mut command = remediation_finding_command(
            saved,
            remediation_finding_admission(&state, round, intent, id)?,
        )?;
        command.admission.claims_request_sha256 =
            ce(claim_evidence::claims_request_sha256(&command))?;
        let locator = ce(owners.claims.claims.locator(&command))?;
        if locator.grant_sha256 != selection.finding_grant_sha256
            || locator.root != selection.root.owner()
        {
            return Err(CampaignError::Conflict(
                "remediation finding grant differs".into(),
            ));
        }
        let registration_id = format!("{id}:register");
        let stage = StageIntent {
            stage_id: id.clone(),
            kind: StageKind::Finding,
            registration_operation_id: registration_id.clone(),
            registration_base_revision: state.revision.clone(),
            command: encode(saved)?,
            request_sha256: command.admission.claims_request_sha256.clone(),
            original_admission: encode(&J2Admission::from(&command.admission))?,
            original_admission_sha256: command.admission.digest(),
            owner_selection_digest: intent.owner_selection_digest.clone(),
            owner_locator: encode(&J2Locator::from(&locator))?,
            evidence_ref: intent.execution_evidence_ref.clone(),
            progress: StageProgress::Registered,
        };
        let digest = campaign_digest(&json!({"completion":completion,"stage":stage}))?;
        let mut next = state.clone();
        remediation_round_mut(&mut next, &completion.round.round_id)?
            .completion
            .as_mut()
            .unwrap()
            .stages
            .push(stage.clone());
        next.revision = state_revision(&next)?;
        if serde_json::to_vec(&next)
            .map_err(|e| CampaignError::Contract(e.to_string()))?
            .len()
            > 8_388_608
        {
            return Err(CampaignError::Contract(
                "remediation finding state exceeds capacity".into(),
            ));
        }
        let receipt = gate.receipt(
            &next,
            Some(state.revision.clone()),
            &registration_id,
            "register_remediation_finding",
            digest,
        );
        gate.store
            .write(&next, Some(&state.revision), &receipt, false, None)?;
        Ok(stage)
    }

    fn publish_remediation_finding(
        &mut self,
        completion: &RemediationCompletionPrepared,
        saved: &RemediationFindingCommand,
        owners: &mut InitialCompletionOwners<'_>,
    ) -> Result<Value> {
        let stage = self.register_remediation_finding(completion, saved, owners)?;
        let locator = decode::<J2Locator>(&stage.owner_locator)?.owner()?;
        let command = remediation_finding_command(saved, binding_for(&stage)?)?;
        if ce(owners.claims.claims.locator(&command))? != locator {
            return Err(CampaignError::Conflict(
                "remediation finding locator differs".into(),
            ));
        }
        let receipt = match owners.claims.claims.reconcile(&locator) {
            Reconciliation::Committed(value) => *value,
            Reconciliation::Absent => match owners.claims.claims.publish(command) {
                Ok(Publication::Applied(value) | Publication::Replayed(value)) => value,
                Err(claim_evidence::WriteError::NoEffect(error)) => {
                    return Err(CampaignError::Conflict(error.to_string()));
                }
                Err(claim_evidence::WriteError::OutcomeUnknown(locator)) => {
                    return Err(CampaignError::Io(format!(
                        "remediation finding uncertain: {locator:?}"
                    )));
                }
            },
            Reconciliation::Conflicting => {
                return Err(CampaignError::Conflict(
                    "remediation finding owner conflicts".into(),
                ));
            }
            Reconciliation::Unresolved(reason) => return Err(CampaignError::Io(reason)),
        };
        let value = remediation_finding_record(&receipt, saved)?;
        if let StageProgress::Checked {
            exact_refs,
            content_digests,
        } = &stage.progress
        {
            if exact_refs.len() != 1
                || content_digests.len() != 1
                || exact_refs[0] != value
                || content_digests[0] != campaign_digest(&value)?
            {
                return Err(CampaignError::Conflict(
                    "saved remediation finding differs".into(),
                ));
            }
        } else {
            match self.record_remediation_stage_progress(
                completion,
                StageKind::Finding,
                Some(&stage.stage_id),
                checked_progress(value.clone())?,
            )? {
                Effect::Applied { .. } | Effect::Replayed { .. } => {}
                Effect::NoEffect(reason) => return Err(CampaignError::Conflict(reason)),
                Effect::OutcomeUnknown(locator) => {
                    return Err(CampaignError::Io(format!(
                        "remediation finding campaign acknowledgement uncertain: {locator:?}"
                    )));
                }
            }
        }
        Ok(value)
    }

    fn register_remediation_projection(
        &mut self,
        completion: &RemediationCompletionPrepared,
        envelope: &ProjectionEnvelope,
        owners: &InitialCompletionOwners<'_>,
    ) -> Result<StageIntent> {
        let mut gate = self.owner()?;
        let state = gate
            .store
            .read(&completion.round.identity)?
            .ok_or_else(|| CampaignError::Integrity("remediation campaign absent".into()))?;
        let round = remediation_round(&state, &completion.round.round_id)?;
        let intent = round
            .completion
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("remediation completion absent".into()))?;
        if round.request.generation != state.remediation.as_ref().unwrap().current_generation
            || intent.campaign_operation_id != completion.completion_operation_id
            || intent.joined_readbacks.is_none()
            || round.outcome.is_some()
        {
            return Err(CampaignError::Conflict(
                "remediation projection registration closed".into(),
            ));
        }
        if let Some(stage) = stage_by_kind(intent, StageKind::FindingProjection) {
            if stage.stage_id != envelope.request_id
                || stage.command != encode(&J2Command::Projection(envelope.clone()))?
            {
                return Err(CampaignError::Conflict(
                    "remediation projection differs".into(),
                ));
            }
            return Ok(stage.clone());
        }
        let selection: CompletionSelectionRecord = decode(&round.completion_selection)?;
        selection.validate()?;
        if owners.claims.claims.root() != &selection.root.owner() {
            return Err(CampaignError::Conflict(
                "remediation projection owner differs".into(),
            ));
        }
        let mut command = projection_command(
            envelope,
            remediation_finding_admission(&state, round, intent, &envelope.request_id)?,
        )?;
        command.admission.claims_request_sha256 =
            ce(claim_evidence::projection_request_sha256(&command))?;
        let stage = StageIntent {
            stage_id: envelope.request_id.clone(),
            kind: StageKind::FindingProjection,
            registration_operation_id: format!("{}:register", envelope.request_id),
            registration_base_revision: state.revision.clone(),
            command: encode(&J2Command::Projection(envelope.clone()))?,
            request_sha256: command.admission.claims_request_sha256.clone(),
            original_admission: encode(&J2Admission::from(&command.admission))?,
            original_admission_sha256: command.admission.digest(),
            owner_selection_digest: intent.owner_selection_digest.clone(),
            owner_locator: encode(&selection.root)?,
            evidence_ref: intent.execution_evidence_ref.clone(),
            progress: StageProgress::Registered,
        };
        let digest = campaign_digest(&json!({"completion":completion,"stage":stage}))?;
        let mut next = state.clone();
        remediation_round_mut(&mut next, &completion.round.round_id)?
            .completion
            .as_mut()
            .unwrap()
            .stages
            .push(stage.clone());
        next.revision = state_revision(&next)?;
        if serde_json::to_vec(&next)
            .map_err(|e| CampaignError::Contract(e.to_string()))?
            .len()
            > 8_388_608
        {
            return Err(CampaignError::Contract(
                "remediation projection exceeds capacity".into(),
            ));
        }
        let receipt = gate.receipt(
            &next,
            Some(state.revision.clone()),
            &stage.registration_operation_id,
            "register_remediation_projection",
            digest,
        );
        gate.store
            .write(&next, Some(&state.revision), &receipt, false, None)?;
        Ok(stage)
    }

    fn read_remediation_projection(
        &mut self,
        completion: &RemediationCompletionPrepared,
        envelope: &ProjectionEnvelope,
        owners: &mut InitialCompletionOwners<'_>,
    ) -> Result<Value> {
        let stage = self.register_remediation_projection(completion, envelope, owners)?;
        let projection = ce(owners
            .claims
            .claims
            .project_exact_revisions(&projection_command(envelope, binding_for(&stage)?)?))?;
        ce(owners.claims.claims.verify_projection_fresh(&projection))?;
        let value = projection_value(&projection)?;
        if let StageProgress::Checked {
            exact_refs,
            content_digests,
        } = &stage.progress
        {
            if exact_refs.len() != 1
                || content_digests.len() != 1
                || exact_refs[0] != value
                || content_digests[0] != campaign_digest(&value)?
            {
                return Err(CampaignError::Conflict(
                    "saved remediation projection differs".into(),
                ));
            }
        } else {
            match self.record_remediation_stage_progress(
                completion,
                StageKind::FindingProjection,
                None,
                checked_progress(value.clone())?,
            )? {
                Effect::Applied { .. } | Effect::Replayed { .. } => {}
                Effect::NoEffect(reason) => return Err(CampaignError::Conflict(reason)),
                Effect::OutcomeUnknown(locator) => {
                    return Err(CampaignError::Io(format!(
                        "remediation projection campaign acknowledgement uncertain: {locator:?}"
                    )));
                }
            }
        }
        Ok(value)
    }

    pub fn finalize_remediation(
        &mut self,
        completion: &RemediationCompletionPrepared,
        outcome_operation_id: &str,
        episode: &mut EpisodeApplication<InProcessReadAdmission>,
        owners: &mut InitialCompletionOwners<'_>,
    ) -> Result<Effect<Value>> {
        require_text(outcome_operation_id, "remediation outcome operation")?;
        match self.complete_remediation(completion, episode, owners)? {
            Effect::Applied { .. } | Effect::Replayed { .. } => {}
            Effect::NoEffect(reason) => return Ok(Effect::NoEffect(reason)),
            Effect::OutcomeUnknown(locator) => return Ok(Effect::OutcomeUnknown(locator)),
        }
        let (round, intent, read, native) = {
            let gate = self.owner()?;
            let state = gate
                .store
                .read(&completion.round.identity)?
                .ok_or_else(|| CampaignError::Integrity("remediation campaign absent".into()))?;
            let round = remediation_round(&state, &completion.round.round_id)?.clone();
            let intent = round
                .completion
                .as_ref()
                .ok_or_else(|| CampaignError::Conflict("remediation completion absent".into()))?
                .clone();
            let stage = stage_by_kind(&intent, StageKind::EpisodeResultRead).ok_or_else(|| {
                CampaignError::Integrity("remediation result target absent".into())
            })?;
            let read = episode
                .read_remediation_checked(&remediation_episode_request(&round, stage)?)
                .map_err(|e| CampaignError::Conflict(e.to_string()))?;
            let current = read
                .state()
                .ok_or_else(|| CampaignError::Conflict("remediation result absent".into()))?;
            let native = current
                .current_result()
                .map_err(|e| CampaignError::Integrity(e.to_string()))?
                .ok_or_else(|| CampaignError::Conflict("remediation result absent".into()))?
                .value;
            (round, intent, read, native)
        };
        let current = read.state().unwrap();
        let selection: CompletionSelectionRecord = decode(&round.completion_selection)?;
        selection.validate()?;
        let established = established(&intent)?;
        let commands =
            remediation_finding_commands(&round, &native, current.value(), outcome_operation_id)?;
        let manifest = RemediationOutcomeManifest {
            operation_id: outcome_operation_id.into(),
            native_result_canonical_json: review_episode_core::codec::canonical_json(&native),
            episode_canonical_json: review_episode_core::codec::canonical_json(current.value()),
            finding_commands: commands.clone(),
            selection: selection.clone(),
            established,
        };
        if let Some(saved) = &round.outcome {
            let stage = stage_by_kind(&intent, StageKind::InitialOutcome).ok_or_else(|| {
                CampaignError::Integrity("remediation outcome registration absent".into())
            })?;
            if stage.command != encode(&manifest)? {
                return Err(CampaignError::Conflict(
                    "remediation outcome replay differs".into(),
                ));
            }
            verify_remediation_finalized_outcome(&round, saved, owners)?;
            let publication = format!("{outcome_operation_id}:publish");
            let receipt = self.owner()?.store.receipt(&publication)?.ok_or_else(|| {
                CampaignError::Integrity("remediation publication receipt absent".into())
            })?;
            if receipt.kind != "publish_remediation_outcome"
                || receipt.request_digest != campaign_digest(saved)?
            {
                return Err(CampaignError::Conflict(
                    "remediation publication receipt differs".into(),
                ));
            }
            return Ok(Effect::Replayed {
                value: saved.clone(),
                receipt,
            });
        }
        self.register_remediation_outcome(completion, &manifest, owners)?;
        let mut findings = Vec::new();
        let projection = if established {
            for saved in &commands {
                findings.push(self.publish_remediation_finding(completion, saved, owners)?);
            }
            if findings.is_empty() {
                let joined = intent.joined_readbacks.as_ref().unwrap();
                let body = json!({"schemaVersion":1,"kind":"no_findings",
                    "remediationEpisodeRef":joined.episode,
                    "generation":round.request.generation,
                    "candidateTree":round.request.candidate.tree,
                    "nativeResultClaimSha256":intent.native_result_claim_sha256,
                    "findingSetSha256":ce(claim_digest(&ClaimValue::Array(vec![])))?,
                    "count":0});
                let digest = ce(claim_digest(&claim_from_value(&body)?))?;
                Some(json!({"kind":"no_findings","owner":"slice-campaign",
                    "reference":format!("no-findings:{digest}"),"body":body}))
            } else {
                let envelope = ProjectionEnvelope {
                    request_id: format!("{outcome_operation_id}:projection"),
                    consumer: selection.consumer.clone(),
                    consumer_tree: selection.consumer_tree.clone(),
                    decision_scope: selection.decision_scope.clone(),
                    revisions: findings
                        .iter()
                        .map(|finding| text_field(finding, "revision"))
                        .collect::<Result<Vec<_>>>()?,
                };
                Some(self.read_remediation_projection(completion, &envelope, owners)?)
            }
        } else {
            None
        };
        let kind = if !established {
            InitialOutcomeKind::EvidenceUnestablished
        } else if current.phase() == review_episode_core::state::Phase::Reported
            || findings.is_empty()
        {
            InitialOutcomeKind::Reported
        } else {
            InitialOutcomeKind::AwaitingBuilder
        };
        let joined = intent.joined_readbacks.as_ref().unwrap();
        let publication = format!("{outcome_operation_id}:publish");
        let mut consumptions = intent.consumption_intents.clone();
        if established {
            consumptions[0].consumed_at_operation_id = Some(publication.clone());
        }
        let outcome = json!({
            "kind":kind,
            "generation":round.request.generation,
            "candidate":round.request.candidate,
            "pair":{"builder":joined.builder,"campaign":joined.campaign,
                "selectedClaims":intent.selected_claims},
            "episode":joined.episode,
            "nativeResultCanonicalJson":manifest.native_result_canonical_json,
            "nativePhase":current.get("phase").as_text()
                .map_err(|e| CampaignError::Integrity(e.to_string()))?
                .to_string_checked().map_err(|e| CampaignError::Integrity(e.to_string()))?,
            "nativeVerdict":native.get("verdict").and_then(|v| v.as_text().ok())
                .map(|s| s.to_string_checked()).transpose()
                .map_err(|e| CampaignError::Integrity(e.to_string()))?,
            "limitationsCanonicalJson":review_episode_core::codec::canonical_json(
                native.get("limitations").ok_or_else(||
                    CampaignError::Integrity("limitations absent".into()))?),
            "unresolvedQuestionsCanonicalJson":review_episode_core::codec::canonical_json(
                current.get("unresolvedQuestions")),
            "nativeFindingCount":commands.len(),
            "findings":findings,"projection":projection,
            "consumptions":consumptions,
            "publicationOperationId":publication,
        });
        let mut gate = self.owner()?;
        let state = gate
            .store
            .read(&completion.round.identity)?
            .ok_or_else(|| CampaignError::Integrity("remediation campaign absent".into()))?;
        let saved = remediation_round(&state, &completion.round.round_id)?;
        if saved.outcome.is_some()
            || saved.request.generation != state.remediation.as_ref().unwrap().current_generation
        {
            return Err(CampaignError::Conflict("remediation outcome moved".into()));
        }
        let mut next = state.clone();
        let target = remediation_round_mut(&mut next, &completion.round.round_id)?;
        target.outcome = Some(outcome.clone());
        if established {
            target.completion.as_mut().unwrap().consumption_intents[0].consumed_at_operation_id =
                Some(publication.clone());
        }
        next.revision = state_revision(&next)?;
        if serde_json::to_vec(&next)
            .map_err(|e| CampaignError::Contract(e.to_string()))?
            .len()
            > 8_388_608
        {
            return Err(CampaignError::Contract(
                "remediation publication exceeds capacity".into(),
            ));
        }
        let digest = campaign_digest(&outcome)?;
        let locator = gate.locator(
            &state.identity,
            &publication,
            "publish_remediation_outcome",
            digest.clone(),
            Some(state.revision.clone()),
        );
        let receipt = gate.receipt(
            &next,
            Some(state.revision.clone()),
            &publication,
            "publish_remediation_outcome",
            digest,
        );
        Ok(
            match gate
                .store
                .write(&next, Some(&state.revision), &receipt, false, None)
            {
                Ok(()) => Effect::Applied {
                    value: outcome,
                    receipt,
                },
                Err(CampaignError::Conflict(e) | CampaignError::Contract(e)) => Effect::NoEffect(e),
                Err(_) => Effect::OutcomeUnknown(locator),
            },
        )
    }

    pub fn complete_remediation(
        &mut self,
        completion: &RemediationCompletionPrepared,
        episode: &mut EpisodeApplication<InProcessReadAdmission>,
        owners: &mut InitialCompletionOwners<'_>,
    ) -> Result<Effect<JoinedReadbacks>> {
        checked_owner_wrapper(self, owners)?;
        for kind in [StageKind::AdmissionBuilder, StageKind::AdmissionCampaign] {
            match self.complete_remediation_stage(completion, kind, owners)? {
                RemediationStageOutcome::Ready {
                    progress: StageProgress::Checked { .. },
                    ..
                } => {}
                RemediationStageOutcome::Ready { .. } => {
                    return Err(CampaignError::Conflict(
                        "remediation admission pending".into(),
                    ));
                }
                RemediationStageOutcome::Unresolved { reason, .. } => {
                    return Err(CampaignError::Io(reason));
                }
                RemediationStageOutcome::Conflicting { reason } => {
                    return Err(CampaignError::Conflict(reason));
                }
            }
        }
        let (state, round, intent, stage, selection) = {
            let gate = self.owner()?;
            let state = gate
                .store
                .read(&completion.round.identity)?
                .ok_or_else(|| CampaignError::Integrity("remediation campaign absent".into()))?;
            let round = remediation_round(&state, &completion.round.round_id)?.clone();
            let intent = round
                .completion
                .as_ref()
                .ok_or_else(|| CampaignError::Conflict("remediation completion absent".into()))?
                .clone();
            let stage = stage_by_kind(&intent, StageKind::EpisodeResultRead)
                .ok_or_else(|| CampaignError::Conflict("remediation result target absent".into()))?
                .clone();
            let selection = gate
                .initial_join
                .as_ref()
                .ok_or_else(|| CampaignError::Conflict("episode owner absent".into()))?
                .selection
                .clone();
            (state, round, intent, stage, selection)
        };
        if round.request.request_digest != completion.round.request_digest
            || intent.campaign_operation_id != completion.completion_operation_id
            || round.subject_read.is_none()
        {
            return Err(CampaignError::Conflict(
                "remediation completion differs".into(),
            ));
        }
        let request = remediation_episode_request(&round, &stage)?;
        let read = episode
            .read_remediation_checked(&request)
            .map_err(|e| CampaignError::Conflict(e.to_string()))?;
        let resolved = read.resolved_revision().ok_or_else(|| {
            CampaignError::Conflict("remediation result transition pending".into())
        })?;
        if matches!(stage.progress, StageProgress::Registered)
            && read.observed_revision() != Some(resolved)
        {
            return Err(CampaignError::Conflict(
                "remediation result is no longer current at first join".into(),
            ));
        }
        let current = read
            .state()
            .ok_or_else(|| CampaignError::Integrity("remediation result state absent".into()))?;
        let content =
            parse_episode_json(text_field(&stage.command, "contentCanonicalJson")?.as_str())
                .map_err(|e| CampaignError::Integrity(e.to_string()))?;
        let expected_result = content
            .get("payload")
            .and_then(|p| p.get("result"))
            .ok_or_else(|| CampaignError::Integrity("result payload absent".into()))?;
        let expected_admissions = content
            .get("payload")
            .and_then(|p| p.get("evidenceAdmissions"))
            .ok_or_else(|| CampaignError::Integrity("admission payload absent".into()))?;
        let actual_result = current
            .current_result()
            .map_err(|e| CampaignError::Integrity(e.to_string()))?
            .ok_or_else(|| CampaignError::Conflict("remediation result absent".into()))?;
        let subject =
            candidate_episode_reference(&selection.episode_authority, &round.request.candidate)?;
        let result_digest = episode_value_digest(expected_result);
        let expected_target = match &request {
            review_episode::in_process_admission::EpisodeReadRequest::Transition {
                transition_id,
                content_digest,
                ..
            } => review_episode::in_process_admission::EpisodeReadTarget::Transition {
                transition_id: transition_id.clone(),
                content_digest: content_digest.clone(),
            },
            _ => {
                return Err(CampaignError::Integrity(
                    "remediation target is not a transition".into(),
                ));
            }
        };
        if read.selection() != &selection.episode_selection
            || read.root() != Path::new(&selection.episode_selection.root)
            || read.identity() != selection.episode_authority.identity()
            || read.authority_digest().0
                != episode_value_digest(selection.episode_authority.value())
            || read.target() != &expected_target
            || current.revision() != resolved
            || current.get("authority") != &selection.episode_authority.binding()
            || current.get("writer") != &selection.episode_authority.writer_value()
            || current.get("subject") != &subject
            || actual_result.value != *expected_result
            || current.get("evidenceAdmissions") != expected_admissions
            || read.result_digest().map(|r| r.0.as_str()) != Some(result_digest.as_str())
            || stage.command["expectedEpisodeDigest"] != result_digest
        {
            return Err(CampaignError::Conflict(
                "remediation result owner read differs".into(),
            ));
        }
        let builder = checked_record_from_stage(
            stage_by_kind(&intent, StageKind::AdmissionBuilder)
                .ok_or_else(|| CampaignError::Conflict("builder admission absent".into()))?,
        )?;
        let terminal = checked_record_from_stage(
            stage_by_kind(&intent, StageKind::AdmissionCampaign)
                .ok_or_else(|| CampaignError::Conflict("terminal admission absent".into()))?,
        )?;
        let episode_record = json!({
            "root":selection.episode_selection.root,
            "selectionDigest":selection.episode_selection.selection_digest,
            "executableSha256":selection.episode_selection.executable_sha256,
            "authorityDigest":read.authority_digest().0,
            "identity":serde_from_episode(&read.identity().value())?,
            "resolvedRevision":resolved.0,
            "observedRevision":read.observed_revision().map(|r| r.0.clone()),
            "resultTransitionIdCanonicalJson":round.frozen_dispatch_command.result_transition_id_canonical_json,
            "contentDigest":stage.request_sha256,
            "episodeResultDigest":result_digest,
            "nativeResultClaimDigest":intent.native_result_claim_sha256,
            "admissionPairDigest":episode_value_digest(expected_admissions),
        });
        if let StageProgress::Checked {
            exact_refs,
            content_digests,
        } = &stage.progress
        {
            let mut stable = episode_record.clone();
            stable["observedRevision"] = exact_refs
                .first()
                .map_or(Value::Null, |saved| saved["observedRevision"].clone());
            if exact_refs.len() != 1
                || content_digests.len() != 1
                || exact_refs[0] != stable
                || content_digests[0] != campaign_digest(&stable)?
            {
                return Err(CampaignError::Conflict(
                    "saved remediation result read differs".into(),
                ));
            }
        } else {
            match self.record_remediation_stage_progress(
                completion,
                StageKind::EpisodeResultRead,
                None,
                StageProgress::Checked {
                    exact_refs: vec![episode_record.clone()],
                    content_digests: vec![campaign_digest(&episode_record)?],
                },
            )? {
                Effect::Applied { .. } | Effect::Replayed { .. } => {}
                Effect::NoEffect(reason) => return Err(CampaignError::Conflict(reason)),
                Effect::OutcomeUnknown(locator) => return Ok(Effect::OutcomeUnknown(locator)),
            }
        }
        let joined = JoinedReadbacks {
            builder,
            campaign: terminal,
            episode: episode_record,
        };
        let mut gate = self.owner()?;
        let current = gate
            .store
            .read(&state.identity)?
            .ok_or_else(|| CampaignError::Integrity("remediation campaign absent".into()))?;
        let saved_intent = remediation_round(&current, &completion.round.round_id)?
            .completion
            .as_ref()
            .ok_or_else(|| CampaignError::Integrity("completion absent".into()))?;
        if let Some(saved) = &saved_intent.joined_readbacks {
            let mut stable = joined.clone();
            stable.episode["observedRevision"] = saved.episode["observedRevision"].clone();
            if serde_json::to_value(saved).ok() != serde_json::to_value(&stable).ok() {
                return Err(CampaignError::Conflict(
                    "remediation joined owners differ".into(),
                ));
            }
            let operation_id = format!("{}:joined", intent.campaign_operation_id);
            let receipt = gate.store.receipt(&operation_id)?.ok_or_else(|| {
                CampaignError::Integrity("remediation join receipt absent".into())
            })?;
            return Ok(Effect::Replayed {
                value: saved.clone(),
                receipt,
            });
        }
        let operation_id = format!("{}:joined", intent.campaign_operation_id);
        let digest = campaign_digest(&json!({"completion":completion,"joined":joined}))?;
        let mut next = current.clone();
        remediation_round_mut(&mut next, &completion.round.round_id)?
            .completion
            .as_mut()
            .unwrap()
            .joined_readbacks = Some(joined.clone());
        next.revision = state_revision(&next)?;
        let locator = gate.locator(
            &current.identity,
            &operation_id,
            "commit_remediation_readbacks",
            digest.clone(),
            Some(current.revision.clone()),
        );
        let receipt = gate.receipt(
            &next,
            Some(current.revision.clone()),
            &operation_id,
            "commit_remediation_readbacks",
            digest,
        );
        Ok(
            match gate
                .store
                .write(&next, Some(&current.revision), &receipt, false, None)
            {
                Ok(()) => Effect::Applied {
                    value: joined,
                    receipt,
                },
                Err(CampaignError::Conflict(e) | CampaignError::Contract(e)) => Effect::NoEffect(e),
                Err(_) => Effect::OutcomeUnknown(locator),
            },
        )
    }
    pub fn register_remediation_episode_read(
        &mut self,
        completion: &RemediationCompletionPrepared,
        expected: &CampaignRevision,
        operation_id: &str,
        result_content: &EpisodeValue,
    ) -> Result<Effect<review_episode::in_process_admission::EpisodeReadRequest>> {
        require_text(operation_id, "remediation result registration")?;
        let content_json = review_episode_core::codec::canonical_json(result_content);
        let content_digest = episode_value_digest(result_content);
        let mut gate = self.owner()?;
        if let Some((receipt, saved)) = gate.store.replay_result(operation_id)? {
            let round = remediation_round(&saved, &completion.round.round_id)?;
            let intent = round.completion.as_ref().ok_or_else(|| {
                CampaignError::Integrity("remediation result registration absent".into())
            })?;
            let stage = stage_by_kind(intent, StageKind::EpisodeResultRead).ok_or_else(|| {
                CampaignError::Integrity("remediation result target absent".into())
            })?;
            if receipt.kind != "register_remediation_episode_read"
                || receipt.prior_revision.as_ref() != Some(expected)
                || stage.registration_operation_id != operation_id
                || stage.command["contentCanonicalJson"] != content_json
                || stage.request_sha256 != content_digest
                || round.request.request_digest != completion.round.request_digest
            {
                return Err(CampaignError::Conflict(
                    "remediation result replay differs".into(),
                ));
            }
            return Ok(Effect::Replayed {
                value: remediation_episode_request(round, stage)?,
                receipt,
            });
        }
        let state = gate.current(&completion.round.identity, expected)?;
        let round = remediation_round(&state, &completion.round.round_id)?;
        let intent = round
            .completion
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("remediation completion absent".into()))?;
        if round.request.request_digest != completion.round.request_digest
            || intent.campaign_operation_id != completion.completion_operation_id
            || round.subject_read.is_none()
            || stage_by_kind(intent, StageKind::EpisodeResultRead).is_some()
        {
            return Err(CampaignError::Conflict(
                "remediation result target differs".into(),
            ));
        }
        let builder = checked_record_from_stage(
            stage_by_kind(intent, StageKind::AdmissionBuilder)
                .ok_or_else(|| CampaignError::Conflict("builder admission pending".into()))?,
        )?;
        let terminal = checked_record_from_stage(
            stage_by_kind(intent, StageKind::AdmissionCampaign)
                .ok_or_else(|| CampaignError::Conflict("terminal admission pending".into()))?,
        )?;
        let payload = result_content
            .get("payload")
            .ok_or_else(|| CampaignError::Contract("remediation result payload absent".into()))?;
        if result_content
            .as_object()
            .map_err(|e| CampaignError::Contract(e.to_string()))?
            .len()
            != 2
            || result_content.get("action") != Some(&EpisodeValue::text("record_result"))
            || payload
                .as_object()
                .map_err(|e| CampaignError::Contract(e.to_string()))?
                .len()
                != 3
        {
            return Err(CampaignError::Contract(
                "remediation result content shape differs".into(),
            ));
        }
        let checked = gate
            .initial_join
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("execution owner absent".into()))?
            .evidence
            .read_result(&intent.execution_evidence_ref.checked()?)
            .map_err(|e| CampaignError::Integrity(e.to_string()))?;
        let native = parse_episode_json(
            std::str::from_utf8(checked.claim_canonical_bytes())
                .map_err(|_| CampaignError::Contract("remediation result bytes invalid".into()))?,
        )
        .map_err(|e| CampaignError::Contract(e.to_string()))?;
        ReviewResult::parse(native.clone()).map_err(|e| CampaignError::Contract(e.to_string()))?;
        if payload.get("result") != Some(&native)
            || checked.claim_sha256() != intent.native_result_claim_sha256
            || checked.raw_sha256() != intent.native_result_raw_sha256
        {
            return Err(CampaignError::Conflict(
                "remediation result differs from E1".into(),
            ));
        }
        review_episode_core::state::validate_text_array(
            payload
                .get("unresolvedQuestions")
                .ok_or_else(|| CampaignError::Contract("remediation questions absent".into()))?,
            "unresolvedQuestions",
        )
        .map_err(|e| CampaignError::Contract(e.to_string()))?;
        let consumptions = [
            consumption_commitment(&builder, operation_id)?,
            consumption_commitment(&terminal, operation_id)?,
        ];
        let expected_admissions = json!([
            episode_admission(&builder, &consumptions[0]),
            episode_admission(&terminal, &consumptions[1]),
        ]);
        if payload.get("evidenceAdmissions")
            != Some(&episode_value_from_serde(&expected_admissions)?)
        {
            return Err(CampaignError::Conflict(
                "remediation admissions differ".into(),
            ));
        }
        for admission in expected_admissions.as_array().unwrap() {
            review_episode_core::state::validate_admission(&episode_value_from_serde(admission)?)
                .map_err(|e| CampaignError::Contract(e.to_string()))?;
        }
        let stage_id = stage_id(&intent.campaign_operation_id, StageKind::EpisodeResultRead);
        let stage = StageIntent {
            stage_id,
            kind: StageKind::EpisodeResultRead,
            registration_operation_id: operation_id.into(),
            registration_base_revision: expected.clone(),
            command: json!({"contentCanonicalJson":content_json,
                "contentDigest":content_digest,
                "expectedClaimDigest":intent.native_result_claim_sha256,
                "expectedEpisodeDigest":episode_value_digest(&native)}),
            request_sha256: content_digest.clone(),
            original_admission: Value::Null,
            original_admission_sha256: content_digest.clone(),
            owner_selection_digest: intent.owner_selection_digest.clone(),
            owner_locator: json!({"transitionIdCanonicalJson":
                round.frozen_dispatch_command.result_transition_id_canonical_json,
                "contentDigest":content_digest}),
            evidence_ref: intent.execution_evidence_ref.clone(),
            progress: StageProgress::Registered,
        };
        let digest = campaign_digest(&json!({"completion":completion,
            "expectedRevision":expected,"stage":stage,"consumptions":consumptions}))?;
        let request = remediation_episode_request(round, &stage)?;
        let mut next = state.clone();
        let intent = remediation_round_mut(&mut next, &completion.round.round_id)?
            .completion
            .as_mut()
            .unwrap();
        intent.consumption_intents = consumptions.to_vec();
        intent.stages.push(stage);
        next.revision = state_revision(&next)?;
        if serde_json::to_vec(&next)
            .map_err(|e| CampaignError::Contract(e.to_string()))?
            .len()
            > 8_388_608
        {
            return Err(CampaignError::Contract(
                "remediation result target exceeds capacity".into(),
            ));
        }
        let locator = gate.locator(
            &state.identity,
            operation_id,
            "register_remediation_episode_read",
            digest.clone(),
            Some(expected.clone()),
        );
        let receipt = gate.receipt(
            &next,
            Some(expected.clone()),
            operation_id,
            "register_remediation_episode_read",
            digest,
        );
        Ok(
            match gate
                .store
                .write(&next, Some(expected), &receipt, false, None)
            {
                Ok(()) => Effect::Applied {
                    value: request,
                    receipt,
                },
                Err(CampaignError::Conflict(e) | CampaignError::Contract(e)) => Effect::NoEffect(e),
                Err(_) => Effect::OutcomeUnknown(locator),
            },
        )
    }
    pub fn confirm_remediation_subject_read(
        &mut self,
        prepared: &RemediationPrepared,
        expected: &CampaignRevision,
        operation_id: &str,
        episode: &review_episode::in_process_admission::CheckedEpisodeReadback,
    ) -> Result<Effect<RemediationSubjectRead>> {
        require_text(operation_id, "remediation subject read operation")?;
        let mut gate = self.owner()?;
        let state = gate.current(&prepared.request.identity, expected)?;
        let round = remediation_round(&state, &prepared.request.round_id)?;
        let dispatch = round
            .dispatch
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("remediation dispatch absent".into()))?;
        let join = gate
            .initial_join
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("episode owner absent".into()))?;
        let authority = &join.selection.episode_authority;
        let subject = candidate_episode_reference(authority, &round.request.candidate)?;
        let content = EpisodeValue::object([
            ("action", EpisodeValue::text("record_remediation_subject")),
            (
                "payload",
                EpisodeValue::object([("subject", subject.clone())]),
            ),
        ]);
        let content_digest = episode_value_digest(&content);
        let transition_id =
            parse_episode_json(&dispatch.command.subject_transition_id_canonical_json)
                .map_err(|e| CampaignError::Contract(e.to_string()))?
                .as_text()
                .map_err(|e| CampaignError::Contract(e.to_string()))?
                .clone();
        let target = review_episode::in_process_admission::EpisodeReadTarget::Transition {
            transition_id,
            content_digest: EpisodeRevision(content_digest.clone()),
        };
        let prior_result = parse_episode_json(&round.predecessor_result_canonical_json)
            .map_err(|e| CampaignError::Integrity(e.to_string()))?;
        let current = episode
            .state()
            .ok_or_else(|| CampaignError::Conflict("remediation subject read absent".into()))?;
        let resolved = episode.resolved_revision().ok_or_else(|| {
            CampaignError::Conflict("remediation subject transition absent".into())
        })?;
        if round.request.request_digest != prepared.request.request_digest
            || round.subject_read.is_some()
            || round.completion.is_some()
            || episode.selection() != &join.selection.episode_selection
            || episode.authority_digest().0 != episode_value_digest(authority.value())
            || episode.identity() != authority.identity()
            || episode.target() != &target
            || episode.observed_revision() != Some(resolved)
            || current.revision() != resolved
            || current.get("authority") != &authority.binding()
            || current.get("writer") != &authority.writer_value()
            || current.get("subject") != &subject
            || current.phase() != review_episode_core::state::Phase::ReEvaluation
            || current.status() != review_episode_core::state::Status::Active
            || current.get("currentResult") != &prior_result
        {
            return Err(CampaignError::Conflict(
                "remediation subject cut differs".into(),
            ));
        }
        let record = RemediationSubjectRead {
            operation_id: operation_id.into(),
            transition_id_canonical_json: dispatch
                .command
                .subject_transition_id_canonical_json
                .clone(),
            content_digest,
            resolved_revision: resolved.0.clone(),
            result_digest: episode
                .result_digest()
                .ok_or_else(|| CampaignError::Conflict("predecessor result digest absent".into()))?
                .0
                .clone(),
            authority_digest: episode.authority_digest().0.clone(),
        };
        let digest = campaign_digest(&json!({"prepared":prepared,"expectedRevision":expected,
            "subjectRead":record}))?;
        let mut next = state.clone();
        remediation_round_mut(&mut next, &prepared.request.round_id)?.subject_read =
            Some(record.clone());
        next.revision = state_revision(&next)?;
        let locator = gate.locator(
            &state.identity,
            operation_id,
            "confirm_remediation_subject_read",
            digest.clone(),
            Some(expected.clone()),
        );
        let receipt = gate.receipt(
            &next,
            Some(expected.clone()),
            operation_id,
            "confirm_remediation_subject_read",
            digest,
        );
        Ok(
            match gate
                .store
                .write(&next, Some(expected), &receipt, false, None)
            {
                Ok(()) => Effect::Applied {
                    value: record,
                    receipt,
                },
                Err(CampaignError::Conflict(e) | CampaignError::Contract(e)) => Effect::NoEffect(e),
                Err(_) => Effect::OutcomeUnknown(locator),
            },
        )
    }

    fn record_remediation_stage_progress(
        &mut self,
        completion: &RemediationCompletionPrepared,
        kind: StageKind,
        target_id: Option<&str>,
        progress: StageProgress,
    ) -> Result<Effect<StageProgress>> {
        let mut gate = self.owner()?;
        let state = gate
            .store
            .read(&completion.round.identity)?
            .ok_or_else(|| CampaignError::Integrity("campaign absent".into()))?;
        let round = remediation_round(&state, &completion.round.round_id)?;
        let intent = round
            .completion
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("remediation completion absent".into()))?;
        if intent.campaign_operation_id != completion.completion_operation_id {
            return Err(CampaignError::Conflict(
                "remediation completion differs".into(),
            ));
        }
        let stage = intent
            .stages
            .iter()
            .find(|stage| stage.kind == kind && target_id.is_none_or(|id| stage.stage_id == id))
            .ok_or_else(|| CampaignError::Conflict("remediation stage absent".into()))?;
        if serde_json::to_value(&stage.progress).ok() == serde_json::to_value(&progress).ok() {
            return Ok(Effect::NoEffect(
                "remediation progress already recorded".into(),
            ));
        }
        if !matches!(
            stage.progress,
            StageProgress::Registered | StageProgress::Unresolved { .. }
        ) {
            return Err(CampaignError::Conflict(
                "remediation stage already settled differently".into(),
            ));
        }
        let operation_id = format!("{}:progress", stage.stage_id);
        let digest = campaign_digest(&json!({"completion":completion,
            "stageId":stage.stage_id,"progress":progress}))?;
        let mut next = state.clone();
        let target = remediation_round_mut(&mut next, &completion.round.round_id)?
            .completion
            .as_mut()
            .unwrap()
            .stages
            .iter_mut()
            .find(|stage| stage.kind == kind && target_id.is_none_or(|id| stage.stage_id == id))
            .unwrap();
        target.progress = progress.clone();
        next.revision = state_revision(&next)?;
        if serde_json::to_vec(&next)
            .map_err(|e| CampaignError::Contract(e.to_string()))?
            .len()
            > 8_388_608
        {
            return Err(CampaignError::Contract(
                "remediation progress exceeds capacity".into(),
            ));
        }
        let locator = gate.locator(
            &state.identity,
            &operation_id,
            "record_remediation_stage",
            digest.clone(),
            Some(state.revision.clone()),
        );
        let receipt = gate.receipt(
            &next,
            Some(state.revision.clone()),
            &operation_id,
            "record_remediation_stage",
            digest,
        );
        Ok(
            match gate
                .store
                .write(&next, Some(&state.revision), &receipt, false, None)
            {
                Ok(()) => Effect::Applied {
                    value: progress,
                    receipt,
                },
                Err(CampaignError::Conflict(e) | CampaignError::Contract(e)) => Effect::NoEffect(e),
                Err(_) => Effect::OutcomeUnknown(locator),
            },
        )
    }

    pub fn complete_remediation_stage(
        &mut self,
        completion: &RemediationCompletionPrepared,
        kind: StageKind,
        owners: &mut InitialCompletionOwners<'_>,
    ) -> Result<RemediationStageOutcome> {
        checked_owner_wrapper(self, owners)?;
        let (state, round, intent, stage, join) = {
            let gate = self.owner()?;
            let state = gate
                .store
                .read(&completion.round.identity)?
                .ok_or_else(|| CampaignError::Integrity("campaign absent".into()))?;
            let round = remediation_round(&state, &completion.round.round_id)?.clone();
            let intent = round
                .completion
                .as_ref()
                .ok_or_else(|| CampaignError::Conflict("remediation completion absent".into()))?
                .clone();
            let stage = intent
                .stages
                .iter()
                .find(|stage| stage.kind == kind)
                .ok_or_else(|| CampaignError::Conflict("remediation stage absent".into()))?
                .clone();
            let join = gate
                .initial_join
                .as_ref()
                .ok_or_else(|| CampaignError::Conflict("remediation owners absent".into()))?
                .selection
                .clone();
            (state, round, intent, stage, join)
        };
        if round.request.request_digest != completion.round.request_digest
            || intent.campaign_operation_id != completion.completion_operation_id
        {
            return Err(CampaignError::Conflict(
                "remediation completion differs".into(),
            ));
        }
        let registration = self
            .owner()?
            .store
            .receipt(&stage.registration_operation_id)?
            .ok_or_else(|| {
                CampaignError::Integrity("remediation stage registration absent".into())
            })?;
        let access = if state.revision == registration.result_revision {
            ProductionPathAccess::Original
        } else {
            ProductionPathAccess::RecoverExact
        };
        let request = stage_request(&stage, access)?;
        let saved_locator = locator_from_value(&stage.owner_locator)?;
        let progress = match kind {
            StageKind::Observation
            | StageKind::EstablishmentBuilder
            | StageKind::EstablishmentCampaign => {
                let observed = match kind {
                    StageKind::Observation => {
                        owners.claims.claims.reconcile_observation(&saved_locator)
                    }
                    _ => owners.claims.claims.reconcile_establishment(
                        &saved_locator,
                        &claim_from_value(stage.command.get("claim").ok_or_else(|| {
                            CampaignError::Integrity("registered claim absent".into())
                        })?)?,
                    ),
                };
                let receipt = match observed {
                    ProductionPathReconciliation::Committed(receipt) => *receipt,
                    ProductionPathReconciliation::Conflicting => {
                        return Ok(RemediationStageOutcome::Conflicting {
                            reason: "CE locator conflicts".into(),
                        });
                    }
                    ProductionPathReconciliation::Unresolved(reason) => {
                        return Ok(RemediationStageOutcome::Unresolved {
                            owner_locator: stage.owner_locator,
                            reason,
                        });
                    }
                    ProductionPathReconciliation::Absent => {
                        if matches!(stage.progress, StageProgress::OwnerCommitted { .. }) {
                            return Ok(RemediationStageOutcome::Conflicting {
                                reason: "saved CE receipt is absent from owner".into(),
                            });
                        }
                        let publication = match request {
                            ProductionPathRequest::RecordObservation(command) => {
                                owners.claims.claims.record_observation(command)
                            }
                            ProductionPathRequest::EstablishClaim(command) => {
                                owners.claims.claims.establish_claim(command)
                            }
                            _ => {
                                return Err(CampaignError::Integrity(
                                    "CE write kind differs".into(),
                                ));
                            }
                        };
                        match publication {
                            Ok(publication) => write_receipt(publication),
                            Err(ProductionPathWriteError::NoEffect(error)) => {
                                return Ok(RemediationStageOutcome::Conflicting {
                                    reason: error.to_string(),
                                });
                            }
                            Err(ProductionPathWriteError::OutcomeUnknown(locator)) => {
                                return Ok(RemediationStageOutcome::Unresolved {
                                    owner_locator: locator_value(&locator),
                                    reason: "CE write acknowledgement uncertain".into(),
                                });
                            }
                        }
                    }
                };
                let mut expected = saved_locator;
                expected.custody_sha256 = receipt.locator.custody_sha256.clone();
                if receipt.locator != expected {
                    return Ok(RemediationStageOutcome::Conflicting {
                        reason: "CE receipt differs from registered locator".into(),
                    });
                }
                StageProgress::OwnerCommitted {
                    receipt: receipt_value(&receipt),
                }
            }
            StageKind::AdmissionBuilder | StageKind::AdmissionCampaign => {
                let ProductionPathRequest::ReadAdmission(command) = request else {
                    return Err(CampaignError::Integrity("CE read kind differs".into()));
                };
                let checked = match owners.claims.claims.read_claim_admission(command) {
                    Ok(value) => value,
                    Err(error) => {
                        return Ok(RemediationStageOutcome::Unresolved {
                            owner_locator: stage.owner_locator,
                            reason: error.to_string(),
                        });
                    }
                };
                let index = if kind == StageKind::AdmissionBuilder {
                    0
                } else {
                    1
                };
                let predecessor = if index == 0 {
                    StageKind::EstablishmentBuilder
                } else {
                    StageKind::EstablishmentCampaign
                };
                let receipt = committed_receipt(&intent, predecessor)?.ok_or_else(|| {
                    CampaignError::Conflict("remediation establishment pending".into())
                })?;
                let record = checked_claim_record(
                    &checked,
                    &intent.selected_claims[index],
                    &receipt,
                    &intent,
                    &join,
                )?;
                StageProgress::Checked {
                    content_digests: vec![campaign_digest(&record)?],
                    exact_refs: vec![record],
                }
            }
            _ => return Err(CampaignError::Contract("not a remediation CE stage".into())),
        };
        if serde_json::to_value(&stage.progress).ok() != serde_json::to_value(&progress).ok() {
            match self.record_remediation_stage_progress(
                completion,
                kind,
                None,
                progress.clone(),
            )? {
                Effect::Applied { .. } | Effect::Replayed { .. } | Effect::NoEffect(_) => {}
                Effect::OutcomeUnknown(locator) => {
                    return Ok(RemediationStageOutcome::Unresolved {
                        owner_locator: serde_json::to_value(locator)
                            .map_err(|e| CampaignError::Contract(e.to_string()))?,
                        reason: "campaign progress acknowledgement uncertain".into(),
                    });
                }
            }
        }
        let state = self
            .read(&completion.round.identity)?
            .ok_or_else(|| CampaignError::Integrity("campaign absent".into()))?;
        let stage = remediation_round(&state, &completion.round.round_id)?
            .completion
            .as_ref()
            .unwrap()
            .stages
            .iter()
            .find(|stage| stage.kind == kind)
            .ok_or_else(|| CampaignError::Integrity("remediation stage absent".into()))?
            .clone();
        Ok(RemediationStageOutcome::Ready {
            stage: Box::new(stage),
            progress,
        })
    }
    pub fn register_remediation_stage(
        &mut self,
        completion: &RemediationCompletionPrepared,
        kind: StageKind,
        owners: &mut InitialCompletionOwners<'_>,
    ) -> Result<Effect<StageIntent>> {
        checked_owner_wrapper(self, owners)?;
        if !matches!(
            kind,
            StageKind::Observation
                | StageKind::EstablishmentBuilder
                | StageKind::EstablishmentCampaign
                | StageKind::AdmissionBuilder
                | StageKind::AdmissionCampaign
        ) {
            return Err(CampaignError::Contract("not a remediation CE stage".into()));
        }
        let evidence = {
            let gate = self.owner()?;
            let state = gate
                .store
                .read(&completion.round.identity)?
                .ok_or_else(|| CampaignError::Integrity("campaign absent".into()))?;
            let round = remediation_round(&state, &completion.round.round_id)?;
            let intent = round
                .completion
                .as_ref()
                .ok_or_else(|| CampaignError::Conflict("remediation completion absent".into()))?;
            Arc::clone(
                &gate
                    .initial_join
                    .as_ref()
                    .ok_or_else(|| CampaignError::Conflict("execution owner absent".into()))?
                    .evidence,
            )
            .read_result(&intent.execution_evidence_ref.checked()?)
            .map_err(|e| CampaignError::Integrity(e.to_string()))?
        };
        let mut gate = self.owner()?;
        let state = gate
            .store
            .read(&completion.round.identity)?
            .ok_or_else(|| CampaignError::Integrity("campaign absent".into()))?;
        let round = remediation_round(&state, &completion.round.round_id)?;
        let intent = round
            .completion
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("remediation completion absent".into()))?;
        if round.request.request_digest != completion.round.request_digest
            || intent.campaign_operation_id != completion.completion_operation_id
            || intent.native_result_claim_sha256 != evidence.claim_sha256()
            || intent.execution_evidence_ref != ExecutionRefRecord::from(evidence.reference())
        {
            return Err(CampaignError::Conflict(
                "remediation completion changed".into(),
            ));
        }
        if intent.stages.iter().any(|stage| stage.kind == kind) {
            return Ok(Effect::NoEffect(
                "remediation stage already registered".into(),
            ));
        }
        let join = gate.initial_join.as_ref().unwrap();
        let request = remediation_ce_request(
            &state,
            round,
            intent,
            kind,
            &evidence,
            &join.selection.production_grant_id,
        )?;
        let owner_locator =
            stage_locator_for_request(&owners.claims.claims, &request, &join.selection)?;
        let binding = match &request {
            ProductionPathRequest::RecordObservation(command) => &command.admission,
            ProductionPathRequest::EstablishClaim(command) => &command.admission,
            ProductionPathRequest::ReadAdmission(command) => &command.admission,
        };
        let stage_id = stage_id(&intent.campaign_operation_id, kind);
        let operation_id = format!("{stage_id}:register");
        let stage = StageIntent {
            stage_id,
            kind,
            registration_operation_id: operation_id.clone(),
            registration_base_revision: state.revision.clone(),
            command: command_value(&request)?,
            request_sha256: production_path_request_sha256(&request)
                .map_err(|e| CampaignError::Contract(e.to_string()))?,
            original_admission: binding_value(binding),
            original_admission_sha256: binding.original_digest(),
            owner_selection_digest: join.digest.clone(),
            owner_locator: locator_value(&owner_locator),
            evidence_ref: intent.execution_evidence_ref.clone(),
            progress: StageProgress::Registered,
        };
        let digest = campaign_digest(&json!({"completion":completion,
            "stage":stage,"expectedRevision":state.revision}))?;
        let mut next = state.clone();
        remediation_round_mut(&mut next, &completion.round.round_id)?
            .completion
            .as_mut()
            .unwrap()
            .stages
            .push(stage.clone());
        next.revision = state_revision(&next)?;
        if serde_json::to_vec(&next)
            .map_err(|e| CampaignError::Contract(e.to_string()))?
            .len()
            > 8_388_608
        {
            return Err(CampaignError::Contract(
                "remediation stage exceeds capacity".into(),
            ));
        }
        let locator = gate.locator(
            &state.identity,
            &operation_id,
            "register_remediation_stage",
            digest.clone(),
            Some(state.revision.clone()),
        );
        let receipt = gate.receipt(
            &next,
            Some(state.revision.clone()),
            &operation_id,
            "register_remediation_stage",
            digest,
        );
        Ok(
            match gate
                .store
                .write(&next, Some(&state.revision), &receipt, false, None)
            {
                Ok(()) => Effect::Applied {
                    value: stage,
                    receipt,
                },
                Err(CampaignError::Conflict(e) | CampaignError::Contract(e)) => Effect::NoEffect(e),
                Err(_) => Effect::OutcomeUnknown(locator),
            },
        )
    }
    pub fn prepare_remediation_completion(
        &mut self,
        prepared: &RemediationPrepared,
        expected: &CampaignRevision,
        operation_id: &str,
        evidence_ref: &review_execution_evidence::ExecutionEvidenceRef,
    ) -> Result<Effect<RemediationCompletionPrepared>> {
        require_text(operation_id, "remediation completion operation")?;
        evidence_ref
            .validate()
            .map_err(|e| CampaignError::Contract(e.to_string()))?;
        {
            let gate = self.owner()?;
            if let Some((receipt, registered)) = gate.store.replay_result(operation_id)? {
                let round = remediation_round(&registered, &prepared.request.round_id)?;
                let intent = round.completion.as_ref().ok_or_else(|| {
                    CampaignError::Integrity("remediation completion replay absent".into())
                })?;
                let digest = campaign_digest(&json!({"prepared":prepared,
                    "expectedRevision":expected,"operationId":operation_id,"intent":intent}))?;
                if receipt.kind != "prepare_remediation_completion"
                    || receipt.identity != prepared.request.identity
                    || receipt.root_id != gate.store.root_id
                    || receipt.prior_revision.as_ref() != Some(expected)
                    || receipt.request_digest != digest
                    || intent.execution_evidence_ref != ExecutionRefRecord::from(evidence_ref)
                    || round.request.request_digest != prepared.request.request_digest
                {
                    return Err(CampaignError::Conflict(
                        "remediation completion replay differs".into(),
                    ));
                }
                return Ok(Effect::Replayed {
                    value: RemediationCompletionPrepared {
                        round: round.request.clone(),
                        completion_operation_id: operation_id.into(),
                        original_registration: gate.locator(
                            &registered.identity,
                            operation_id,
                            "prepare_remediation_completion",
                            digest,
                            Some(expected.clone()),
                        ),
                    },
                    receipt,
                });
            }
        }
        let evidence = {
            let gate = self.owner()?;
            let installed = gate
                .initial_join
                .as_ref()
                .ok_or_else(|| CampaignError::Conflict("execution owner not installed".into()))?;
            Arc::clone(&installed.evidence)
        };
        let checked = evidence
            .read_result(evidence_ref)
            .map_err(|e| CampaignError::Integrity(e.to_string()))?;
        let mut gate = self.owner()?;
        let state = gate.current(&prepared.request.identity, expected)?;
        let installed = gate
            .initial_join
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("execution owner not installed".into()))?;
        installed
            .selection
            .evidence_profile
            .require_class(checked.evidence_class())
            .map_err(|e| CampaignError::Unsupported(e.to_string()))?;
        let round = remediation_round(&state, &prepared.request.round_id)?;
        let dispatch = round
            .dispatch
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("remediation dispatch absent".into()))?;
        let dispatch_receipt = gate.store.receipt(&dispatch.operation_id)?.ok_or_else(|| {
            CampaignError::Integrity("remediation dispatch receipt absent".into())
        })?;
        let binding = &checked.provenance().binding;
        if round.request.request_digest != prepared.request.request_digest
            || round.request.operation_id != prepared.request.operation_id
            || round.request.prepared_revision != prepared.request.prepared_revision
            || round.subject_read.is_none()
            || round.completion.is_some()
            || round.outcome.is_some()
            || evidence_ref != checked.reference()
            || evidence_ref.owner != installed.selection.evidence_owner
            || evidence_ref.root_id != installed.selection.evidence_root_id
            || evidence_ref.profile != installed.selection.evidence_record_profile
            || evidence_ref.attempt_id != round.request.execution_attempt_id
            || binding.campaign_root_id != gate.store.root_id
            || binding.obligation_id != round.request.obligation_id
            || binding.candidate_digest != round.request.candidate.receipt_sha256
            || binding.selection_digest != round.request.selection.campaign_digest
            || binding.profile_digest != round.request.profile_digest
            || binding.prepared_request_sha256 != round.request.request_digest
            || binding.review_episode_id != dispatch.command.episode_id
            || binding.attempt_id != round.request.execution_attempt_id
            || checked.provenance().dispatch_operation_id != dispatch.operation_id
            || checked.provenance().dispatch_revision != dispatch_receipt.result_revision.as_str()
            || checked.provenance().execution_session_id.as_deref()
                != Some(round.request.session.session_id.as_str())
            || gate
                .store
                .dispatch_slot(&state.identity, &round.request.obligation_id)?
                != Some((Some(dispatch.operation_id.clone()), true))
        {
            return Err(CampaignError::Conflict(
                "remediation execution provenance differs".into(),
            ));
        }
        let native = std::str::from_utf8(checked.claim_canonical_bytes())
            .map_err(|_| CampaignError::Contract("remediation result bytes invalid".into()))?;
        let result = ReviewResult::parse(
            parse_episode_json(native).map_err(|e| CampaignError::Contract(e.to_string()))?,
        )
        .map_err(|e| CampaignError::Contract(e.to_string()))?;
        let subject = EpisodeValue::object([
            (
                "commit",
                EpisodeValue::text(&round.request.candidate.commit),
            ),
            ("tree", EpisodeValue::text(&round.request.candidate.tree)),
            (
                "patchIdentity",
                EpisodeValue::text(&round.request.candidate.patch_identity),
            ),
        ]);
        let findings = result
            .value
            .get("findings")
            .ok_or_else(|| CampaignError::Integrity("remediation result findings absent".into()))?
            .as_array()
            .map_err(|e| CampaignError::Contract(e.to_string()))?;
        if result.value.get("subject") != Some(&subject) || findings.len() > 100 {
            return Err(CampaignError::Contract(
                "remediation result subject or capacity differs".into(),
            ));
        }
        let mut native_ids = std::collections::BTreeSet::new();
        for finding in findings {
            let id = finding
                .get("id")
                .ok_or_else(|| CampaignError::Contract("remediation finding ID absent".into()))?;
            let canonical = review_episode_core::codec::canonical_json(id);
            ce(claim_digest(&cp(&canonical)?))?;
            if !native_ids.insert(canonical) {
                return Err(CampaignError::Contract(
                    "duplicate remediation finding ID".into(),
                ));
            }
        }
        for prior in &round.predecessor_finding_refs {
            let raw = text_field(prior, "nativeIdCanonicalJson")?;
            let native =
                parse_episode_json(&raw).map_err(|e| CampaignError::Integrity(e.to_string()))?;
            if !native_ids.contains(&review_episode_core::codec::canonical_json(&native)) {
                return Err(CampaignError::Conflict(
                    "retained result dropped or rewrote a prior finding".into(),
                ));
            }
        }
        let observation_selection = match checked.observation() {
            CheckedObservation::Present(value) => {
                claim_evidence::validate_production_path_observation(value)
                    .map_err(|e| CampaignError::Contract(e.to_string()))?;
                let execution = value.get("execution").ok_or_else(|| {
                    CampaignError::Contract("remediation observation execution absent".into())
                })?;
                let selection = value.get("selection").ok_or_else(|| {
                    CampaignError::Contract("remediation observation selection absent".into())
                })?;
                let observed_subject = value.get("subject").ok_or_else(|| {
                    CampaignError::Contract("remediation observation subject absent".into())
                })?;
                let continuity = value.get("continuity").ok_or_else(|| {
                    CampaignError::Contract("remediation continuity absent".into())
                })?;
                if claim_text_field(execution, "resultDigest")? != checked.claim_sha256()
                    || claim_text_field(execution, "attemptId")?
                        != round.request.execution_attempt_id
                    || claim_text_field(selection, "revision")?
                        != round.request.selection.campaign_digest
                    || claim_text_field(observed_subject, "reviewEpisodeId")?
                        != dispatch.command.episode_id
                    || claim_text_field(value, "obligationId")? != round.request.obligation_id
                    || claim_text_field(continuity, "mode")? != "same_session_resume"
                    || claim_text_field(continuity, "sessionId")?
                        != round.request.session.session_id
                {
                    return Err(CampaignError::Conflict(
                        "remediation observation differs".into(),
                    ));
                }
                ObservationSelection::Present {
                    observation_id: claim_text_field(value, "id")?,
                    event_identity: claim_text_field(value, "event_identity")?,
                    observation_sha256: ce(claim_digest(value))?,
                }
            }
            CheckedObservation::Absent { reason } => ObservationSelection::Absent {
                reason: reason.clone(),
                evidence_ref: ExecutionRefRecord::from(evidence_ref),
            },
        };
        let intent = CompletionIntent {
            campaign_operation_id: operation_id.into(),
            registration_operation_id: operation_id.into(),
            registration_base_revision: expected.clone(),
            obligation_id: round.request.obligation_id.clone(),
            attempt_id: round.request.execution_attempt_id.clone(),
            preparation_operation_id: round.request.operation_id.clone(),
            prepared_request_sha256: round.request.request_digest.clone(),
            dispatch_operation_id: dispatch.operation_id.clone(),
            owner_selection_digest: installed.digest.clone(),
            execution_evidence_ref: ExecutionRefRecord::from(evidence_ref),
            native_result_claim_sha256: checked.claim_sha256().into(),
            native_result_raw_sha256: checked.raw_sha256().into(),
            execution_session_id: checked.provenance().execution_session_id.clone(),
            observation_selection,
            selected_claims: round.accepted_claims.clone(),
            stages: Vec::new(),
            consumption_intents: Vec::new(),
            joined_readbacks: None,
            outcome: None,
        };
        let digest = campaign_digest(&json!({"prepared":prepared,"expectedRevision":expected,
            "operationId":operation_id,"intent":intent}))?;
        let mut next = state.clone();
        remediation_round_mut(&mut next, &prepared.request.round_id)?.completion = Some(intent);
        next.revision = state_revision(&next)?;
        if serde_json::to_vec(&next)
            .map_err(|e| CampaignError::Contract(e.to_string()))?
            .len()
            > 8_388_608
        {
            return Err(CampaignError::Contract(
                "remediation completion exceeds capacity".into(),
            ));
        }
        let locator = gate.locator(
            &next.identity,
            operation_id,
            "prepare_remediation_completion",
            digest.clone(),
            Some(expected.clone()),
        );
        let value = RemediationCompletionPrepared {
            round: prepared.request.clone(),
            completion_operation_id: operation_id.into(),
            original_registration: locator.clone(),
        };
        let receipt = gate.receipt(
            &next,
            Some(expected.clone()),
            operation_id,
            "prepare_remediation_completion",
            digest,
        );
        Ok(
            match gate
                .store
                .write(&next, Some(expected), &receipt, false, None)
            {
                Ok(()) => Effect::Applied { value, receipt },
                Err(CampaignError::Conflict(e) | CampaignError::Contract(e)) => Effect::NoEffect(e),
                Err(_) => Effect::OutcomeUnknown(locator),
            },
        )
    }
    /// Bind one changed candidate to a settled predecessor and the exact
    /// currently checked episode result. No provider entry occurs here.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_remediation(
        &mut self,
        identity: &CampaignIdentity,
        expected: &CampaignRevision,
        obligation_id: &str,
        round_id: &str,
        operation_id: &str,
        initial_outcome: &InitialOutcomeLocator,
        selection: RemediationSelection,
        episode: &review_episode::in_process_admission::CheckedEpisodeReadback,
    ) -> Result<RemediationPreparation> {
        require_text(obligation_id, "remediation obligation")?;
        require_text(operation_id, "remediation operation")?;
        if self
            .locate_remediation(identity, obligation_id, round_id, operation_id)?
            .is_none()
        {
            validate_outcome_locator(self, initial_outcome)?;
        }
        let mut gate = self.owner()?;
        gate.check_identity(identity)?;
        if gate.config.schema_version != 4 {
            return Ok(RemediationPreparation::NoEffect(
                "schema 3 is initial-only".into(),
            ));
        }
        let mut state = gate
            .store
            .read(identity)?
            .ok_or_else(|| CampaignError::Conflict("campaign absent".into()))?;
        let next_generation = state
            .remediation
            .as_ref()
            .map_or(1, |p| p.current_generation + 1);
        let digest = campaign_digest(&json!({
            "identity":identity,"expectedRevision":expected,"obligationId":obligation_id,
            "roundId":round_id,"operationId":operation_id,
            "initialOutcome":initial_outcome,
            "candidateRawSha256":raw_sha256(&selection.parts.candidate_raw),
            "profileRawSha256":raw_sha256(&selection.parts.profile_raw),
            "gateCaptureSha256":raw_sha256(&selection.parts.gate_capture),
            "gateManifestSha256":raw_sha256(&selection.parts.gate_manifest),
            "gateReceiptSha256":raw_sha256(&selection.parts.gate_receipt),
            "selection":selection.parts.selection,"completion":selection.parts.completion.record,
            "session":selection.parts.session,"dispatchCommand":selection.parts.dispatch_command,
            "acceptanceSources":selection.parts.acceptance_sources,
            "episodeRevision":episode.resolved_revision().map(|r| r.0.as_str()),
            "episodeResult":episode.result_digest().map(|r| r.0.as_str()),
        }))?;
        if let Some((receipt, replayed)) =
            gate.replay(operation_id, "prepare_remediation", &digest, identity)?
        {
            let round = remediation_round(&replayed, round_id)?;
            return Ok(RemediationPreparation::Replayed {
                prepared: RemediationPrepared {
                    request: round.request.clone(),
                    original_registration: gate.locator(
                        identity,
                        operation_id,
                        "prepare_remediation",
                        digest,
                        Some(expected.clone()),
                    ),
                },
                receipt,
            });
        }
        if round_id != format!("generation-{next_generation}") {
            return Err(CampaignError::Contract(
                "round ID differs from generation".into(),
            ));
        }
        if state.revision != *expected {
            return Err(CampaignError::Conflict("expected revision differs".into()));
        }
        let installed = gate.initial_join.as_ref().ok_or_else(|| {
            CampaignError::Contract("episode and CE owner installation absent".into())
        })?;
        let initial_intent = state
            .progress
            .attempts
            .iter()
            .filter_map(|attempt| attempt.completion_intent.as_ref())
            .find(|intent| intent.obligation_id == obligation_id)
            .ok_or_else(|| CampaignError::Integrity("initial completion absent".into()))?;
        if selection.parts.session.owner != initial_intent.execution_evidence_ref.owner
            || selection.parts.session.root_id != initial_intent.execution_evidence_ref.root_id
            || selection.parts.session.initial_attempt_id != initial_intent.attempt_id
            || initial_intent.execution_session_id.as_deref()
                != Some(selection.parts.session.session_id.as_str())
            || (next_generation > 1
                && state.remediation.as_ref().unwrap().rounds[0]
                    .request
                    .session
                    != selection.parts.session)
        {
            return Err(CampaignError::Contract(
                "retained session differs from initial checked execution".into(),
            ));
        }
        let prior_candidate = if next_generation == 1 {
            state
                .candidate
                .as_ref()
                .ok_or_else(|| CampaignError::Integrity("initial candidate absent".into()))?
        } else {
            let previous = state
                .remediation
                .as_ref()
                .unwrap()
                .rounds
                .last()
                .ok_or_else(|| CampaignError::Integrity("predecessor round absent".into()))?;
            if previous.outcome.is_none()
                || previous
                    .evaluations
                    .iter()
                    .any(|evaluation| evaluation.publication_state != "committed")
            {
                return Ok(RemediationPreparation::NoEffect(
                    "predecessor round is unsettled".into(),
                ));
            }
            &previous.request.candidate
        };
        let prior_tree = prior_candidate.tree.clone();
        let previous_commit = prior_candidate.commit.clone();
        if episode.selection() != &installed.selection.episode_selection
            || episode.authority_digest().0
                != episode_value_digest(installed.selection.episode_authority.value())
            || episode.identity() != installed.selection.episode_authority.identity()
            || episode.observed_revision() != episode.resolved_revision()
        {
            return Err(CampaignError::Contract(
                "episode owner or current revision differs".into(),
            ));
        }
        let episode_state = episode
            .state()
            .ok_or_else(|| CampaignError::Contract("episode absent".into()))?;
        if !matches!(
            episode_state.phase(),
            review_episode_core::state::Phase::Remediation
                | review_episode_core::state::Phase::Reported
        ) || episode_state.status() != review_episode_core::state::Status::Active
            || episode_state.get("authority") != &installed.selection.episode_authority.binding()
            || installed.selection.episode_authority.initial_subject()
                != &candidate_episode_reference(
                    &installed.selection.episode_authority,
                    state.candidate.as_ref().unwrap(),
                )?
        {
            return Err(CampaignError::Contract(
                "episode phase or original authority differs".into(),
            ));
        }
        let subject =
            candidate_episode_reference(&installed.selection.episode_authority, prior_candidate)?;
        if episode_state.get("subject") != &subject {
            return Err(CampaignError::Contract(
                "episode predecessor subject differs".into(),
            ));
        }
        let prior_result = episode_state
            .current_result()
            .map_err(|e| CampaignError::Contract(e.to_string()))?
            .ok_or_else(|| CampaignError::Contract("predecessor result absent".into()))?;
        let prior_result_json = review_episode_core::codec::canonical_json(&prior_result.value);
        let prior_findings = prior_result
            .value
            .get("findings")
            .ok_or_else(|| CampaignError::Integrity("predecessor findings absent".into()))?
            .as_array()
            .map_err(|e| CampaignError::Integrity(e.to_string()))?;
        if prior_findings.len() > 100 {
            return Err(CampaignError::Contract(
                "predecessor finding set exceeds capacity".into(),
            ));
        }
        if next_generation == 1 {
            let intent = initial_intent;
            if !matches!(
                intent.outcome,
                Some(InitialOutcome::AwaitingBuilder { .. })
                    | Some(InitialOutcome::Reported { .. })
            ) || state
                .progress
                .evaluations
                .iter()
                .any(|e| e.publication_state != "committed")
                || (matches!(intent.outcome, Some(InitialOutcome::Reported { .. }))
                    && !prior_findings.is_empty())
            {
                return Ok(RemediationPreparation::NoEffect(
                    "predecessor generation is not eligible".into(),
                ));
            }
            let found = checked_initial_outcome_manifest(&state, obligation_id, initial_outcome)?;
            if found.native_result_canonical_json != prior_result_json {
                return Err(CampaignError::Contract(
                    "initial result and episode differ".into(),
                ));
            }
        } else {
            let predecessor = state.remediation.as_ref().unwrap().rounds.last().unwrap();
            let outcome = predecessor.outcome.as_ref().unwrap();
            if outcome["nativeResultCanonicalJson"] != prior_result_json
                || outcome["nativeFindingCount"] != prior_findings.len()
            {
                return Err(CampaignError::Contract(
                    "predecessor outcome and current episode result differ".into(),
                ));
            }
            if outcome["kind"] != "awaiting_builder"
                && !(outcome["kind"] == "reported" && prior_findings.is_empty())
            {
                return Ok(RemediationPreparation::NoEffect(
                    "predecessor generation is not eligible".into(),
                ));
            }
        }
        let mut baseline = state.baseline.clone();
        baseline.inter_slice_commit = previous_commit;
        let verified = crate::subject_binding::verify_existing_subject_dynamic(
            &gate.config.repository,
            identity,
            &baseline,
            &selection.parts.candidate_raw,
            &selection.parts.profile_raw,
            &selection.parts.gate_capture,
            &selection.parts.gate_manifest,
            &selection.parts.gate_receipt,
        )?;
        if verified.reference.tree == prior_tree {
            return Err(CampaignError::Contract(
                "remediation candidate tree is unchanged".into(),
            ));
        }
        let selection_ref =
            validate_selection(&selection.parts.selection, identity, &verified.reference)?;
        if state
            .selection_ref
            .as_ref()
            .is_some_and(|s| s.selection_id == selection_ref.selection_id)
            || state.remediation.as_ref().is_some_and(|p| {
                p.rounds
                    .iter()
                    .any(|r| r.request.selection.selection_id == selection_ref.selection_id)
            })
        {
            return Err(CampaignError::Contract(
                "successor selection is not fresh".into(),
            ));
        }
        let specialist = selection.parts.selection["specialists"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["obligationId"] == obligation_id && s["selection"] == "selected")
            .ok_or_else(|| CampaignError::Contract("successor obligation differs".into()))?;
        let claims = specialist["requiredClaims"].as_array().unwrap();
        if claims.len() != 2
            || claims
                .iter()
                .any(|c| c["profile"]["continuity"] != "retained")
        {
            return Err(CampaignError::Contract(
                "successor claim continuity differs".into(),
            ));
        }
        let initial_claims = state.review_selection.as_ref().unwrap()["specialists"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["obligationId"] == obligation_id)
            .unwrap()["requiredClaims"]
            .as_array()
            .unwrap();
        if claims.iter().any(|c| {
            initial_claims
                .iter()
                .any(|old| old["claimId"] == c["claimId"])
        }) {
            return Err(CampaignError::Contract(
                "successor claim copied initial authority".into(),
            ));
        }
        if selection.parts.completion.record.consumer_tree != verified.reference.tree
            || selection.parts.completion.record.root.owner() != installed.selection.claims_root
            || selection.parts.session.initial_attempt_id != identity.attempt_id
        {
            return Err(CampaignError::Contract(
                "successor CE or initial attempt binding differs".into(),
            ));
        }
        let accepted_claims = claims
            .iter()
            .map(|c| {
                Ok(SelectedClaim {
                    claim_id: c["claimId"].as_str().unwrap().into(),
                    revision: c["revision"].as_str().unwrap().into(),
                    document_sha256: ce(claim_digest(&claim_from_value(c)?))?,
                    boundary: c["consumptionBoundary"].as_str().unwrap().into(),
                    consumer: c["consumer"].as_str().unwrap().into(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let profile_digest = verified.profile_digest;
        let mut request = RemediationRequestRef {
            root_id: gate.store.root_id.clone(),
            identity: identity.clone(),
            obligation_id: obligation_id.into(),
            round_id: round_id.into(),
            generation: next_generation,
            operation_id: operation_id.into(),
            execution_attempt_id: format!("{}:remediation:{}", identity.key(), round_id),
            prepared_revision: expected.clone(),
            request_digest: String::new(),
            candidate: verified.reference,
            selection: selection_ref,
            profile_digest,
            session: selection.parts.session.clone(),
        };
        request.request_digest = campaign_digest(
            &json!({"rootId":request.root_id,"identity":request.identity,
            "obligationId":request.obligation_id,"roundId":request.round_id,"generation":request.generation,
            "operationId":request.operation_id,"executionAttemptId":request.execution_attempt_id,
            "candidate":request.candidate,"selection":request.selection,
            "profileDigest":request.profile_digest,"session":request.session,
            "dispatchCommand":selection.parts.dispatch_command}),
        )?;
        selection.parts.dispatch_command.validate(&request)?;
        let initial_dispatch = state
            .progress
            .attempts
            .iter()
            .filter_map(|a| a.dispatch.as_ref())
            .find(|d| d.command.episode_id == selection.parts.dispatch_command.episode_id)
            .ok_or_else(|| CampaignError::Integrity("initial dispatch absent".into()))?;
        if selection.parts.dispatch_command.writer_actor != initial_dispatch.command.writer_actor {
            return Err(CampaignError::Contract(
                "remediation writer differs from initial episode".into(),
            ));
        }
        let used_native_ids = [
            review_episode_core::codec::canonical_json(&EpisodeValue::text(
                &initial_dispatch.command.begin_transition_id,
            )),
            review_episode_core::codec::canonical_json(&EpisodeValue::text(
                &initial_dispatch.command.result_transition_id,
            )),
        ];
        if [
            selection
                .parts
                .dispatch_command
                .subject_transition_id_canonical_json
                .as_str(),
            selection
                .parts
                .dispatch_command
                .result_transition_id_canonical_json
                .as_str(),
        ]
        .iter()
        .any(|id| {
            used_native_ids.iter().any(|used| used == id)
                || state.remediation.as_ref().is_some_and(|p| {
                    p.rounds.iter().any(|r| {
                        r.frozen_dispatch_command
                            .subject_transition_id_canonical_json
                            == **id
                            || r.frozen_dispatch_command
                                .result_transition_id_canonical_json
                                == **id
                    })
                })
        }) {
            return Err(CampaignError::Contract(
                "native transition ID already used".into(),
            ));
        }
        let predecessor_refs = if next_generation == 1 {
            let intent = state
                .progress
                .attempts
                .iter()
                .filter_map(|a| a.completion_intent.as_ref())
                .find(|i| i.obligation_id == obligation_id)
                .unwrap();
            intent
                .stages
                .iter()
                .filter(|s| s.kind == StageKind::Finding)
                .map(checked_ref)
                .collect::<Result<Vec<_>>>()?
        } else {
            state
                .remediation
                .as_ref()
                .unwrap()
                .rounds
                .last()
                .unwrap()
                .outcome
                .as_ref()
                .unwrap()["findings"]
                .as_array()
                .cloned()
                .ok_or_else(|| CampaignError::Integrity("predecessor finding refs absent".into()))?
        };
        if predecessor_refs.len() != prior_findings.len() {
            return Err(CampaignError::Integrity(
                "complete predecessor finding set differs".into(),
            ));
        }
        let mut predecessor_ids = predecessor_refs
            .iter()
            .map(|reference| {
                Ok(review_episode_core::codec::canonical_json(
                    &parse_episode_json(&text_field(reference, "nativeIdCanonicalJson")?)
                        .map_err(|e| CampaignError::Integrity(e.to_string()))?,
                ))
            })
            .collect::<Result<std::collections::BTreeSet<_>>>()?;
        if predecessor_ids.len() != predecessor_refs.len() {
            return Err(CampaignError::Integrity(
                "duplicate predecessor finding identity".into(),
            ));
        }
        for finding in prior_findings {
            let id = finding
                .get("id")
                .ok_or_else(|| CampaignError::Integrity("finding ID absent".into()))?;
            if !predecessor_ids.remove(&review_episode_core::codec::canonical_json(id)) {
                return Err(CampaignError::Integrity(
                    "predecessor finding identity differs".into(),
                ));
            }
        }
        if !predecessor_ids.is_empty() {
            return Err(CampaignError::Integrity(
                "predecessor finding identity differs".into(),
            ));
        }
        let round = RemediationRound {
            request: request.clone(),
            initial_outcome: initial_outcome.clone(),
            predecessor_generation: next_generation - 1,
            predecessor_episode_revision: episode_state.revision().0.clone(),
            predecessor_result_canonical_json: prior_result_json,
            predecessor_finding_refs: predecessor_refs,
            candidate_receipt_raw: String::from_utf8(selection.parts.candidate_raw).unwrap(),
            physical_profile_raw: String::from_utf8(selection.parts.profile_raw).unwrap(),
            gate_capture_raw: String::from_utf8(selection.parts.gate_capture).unwrap(),
            gate_manifest_raw: String::from_utf8(selection.parts.gate_manifest).unwrap(),
            gate_receipt_raw: String::from_utf8(selection.parts.gate_receipt).unwrap(),
            accepted_selection: selection.parts.selection,
            accepted_claims,
            frozen_dispatch_command: selection.parts.dispatch_command,
            completion_selection: encode(&selection.parts.completion.record)?,
            acceptance_sources: selection.parts.acceptance_sources,
            dispatch: None,
            subject_read: None,
            completion: None,
            outcome: None,
            evaluations: Vec::new(),
        };
        let progress = state
            .remediation
            .get_or_insert_with(RemediationProgress::default);
        progress.current_generation = next_generation;
        progress.rounds.push(round);
        state.revision = state_revision(&state)?;
        let encoded =
            serde_json::to_vec(&state).map_err(|e| CampaignError::Contract(e.to_string()))?;
        if encoded.len() > 8_388_608 {
            return Err(CampaignError::Contract(
                "remediation state exceeds capacity".into(),
            ));
        }
        state
            .remediation
            .as_mut()
            .unwrap()
            .rounds
            .last_mut()
            .unwrap()
            .request
            .prepared_revision = state.revision.clone();
        let locator = gate.locator(
            identity,
            operation_id,
            "prepare_remediation",
            digest.clone(),
            Some(expected.clone()),
        );
        let receipt = gate.receipt(
            &state,
            Some(expected.clone()),
            operation_id,
            "prepare_remediation",
            digest.clone(),
        );
        match gate
            .store
            .write(&state, Some(expected), &receipt, false, None)
        {
            Ok(()) => Ok(RemediationPreparation::Applied {
                prepared: RemediationPrepared {
                    request: state
                        .remediation
                        .as_ref()
                        .unwrap()
                        .rounds
                        .last()
                        .unwrap()
                        .request
                        .clone(),
                    original_registration: locator,
                },
                receipt,
            }),
            Err(CampaignError::Conflict(e) | CampaignError::Contract(e)) => {
                Ok(RemediationPreparation::NoEffect(e))
            }
            Err(_) => Ok(RemediationPreparation::OutcomeUnknown(locator)),
        }
    }

    pub fn locate_remediation(
        &self,
        identity: &CampaignIdentity,
        obligation_id: &str,
        round_id: &str,
        operation_id: &str,
    ) -> Result<Option<RemediationPrepared>> {
        let gate = self.owner()?;
        gate.check_identity(identity)?;
        let Some(state) = gate.store.read(identity)? else {
            return Ok(None);
        };
        let Some(round) = state.remediation.as_ref().and_then(|p| {
            p.rounds.iter().find(|r| {
                r.request.round_id == round_id
                    && r.request.obligation_id == obligation_id
                    && r.request.operation_id == operation_id
            })
        }) else {
            return Ok(None);
        };
        let receipt = gate.store.receipt(operation_id)?.ok_or_else(|| {
            CampaignError::Integrity("remediation preparation receipt absent".into())
        })?;
        if receipt.kind != "prepare_remediation"
            || receipt.result_revision != round.request.prepared_revision
            || receipt.identity != *identity
        {
            return Err(CampaignError::Integrity(
                "remediation preparation receipt differs".into(),
            ));
        }
        Ok(Some(RemediationPrepared {
            request: round.request.clone(),
            original_registration: gate.locator(
                identity,
                operation_id,
                "prepare_remediation",
                receipt.request_digest,
                receipt.prior_revision,
            ),
        }))
    }

    pub fn recover_remediation(
        &self,
        prepared: &RemediationPrepared,
    ) -> Result<RemediationRecovery> {
        let found = self
            .locate_remediation(
                &prepared.request.identity,
                &prepared.request.obligation_id,
                &prepared.request.round_id,
                &prepared.request.operation_id,
            )?
            .ok_or_else(|| CampaignError::Conflict("remediation preparation absent".into()))?;
        if encode(&found)? != encode(prepared)? {
            return Err(CampaignError::Conflict(
                "remediation locator differs".into(),
            ));
        }
        let gate = self.owner()?;
        let state = gate
            .store
            .read(&prepared.request.identity)?
            .ok_or_else(|| CampaignError::Integrity("campaign absent".into()))?;
        let round = remediation_round(&state, &prepared.request.round_id)?;
        let completion = if let Some(intent) = &round.completion {
            let receipt = gate
                .store
                .receipt(&intent.campaign_operation_id)?
                .ok_or_else(|| {
                    CampaignError::Integrity("remediation completion receipt absent".into())
                })?;
            Some(RemediationCompletionPrepared {
                round: round.request.clone(),
                completion_operation_id: intent.campaign_operation_id.clone(),
                original_registration: gate.locator(
                    &state.identity,
                    &intent.campaign_operation_id,
                    "prepare_remediation_completion",
                    receipt.request_digest,
                    Some(intent.registration_base_revision.clone()),
                ),
            })
        } else {
            None
        };
        Ok(RemediationRecovery {
            prepared: found,
            dispatch: round.dispatch.clone(),
            subject_read: round.subject_read.clone(),
            completion,
            stages: round
                .completion
                .as_ref()
                .map_or_else(Vec::new, |intent| intent.stages.clone()),
            joined_readbacks: round
                .completion
                .as_ref()
                .and_then(|intent| intent.joined_readbacks.clone()),
            outcome: round.outcome.clone(),
        })
    }

    pub fn authorize_remediation_dispatch(
        &mut self,
        prepared: &RemediationPrepared,
        operation_id: &str,
        command: RemediationDispatchCommand,
    ) -> Result<RemediationDispatchEffect> {
        require_text(operation_id, "remediation dispatch operation")?;
        command.validate(&prepared.request)?;
        if operation_id == prepared.request.operation_id {
            return Ok(RemediationDispatchEffect::NoEffect(
                "dispatch identity overlaps preparation".into(),
            ));
        }
        let mut gate = self.owner()?;
        gate.check_identity(&prepared.request.identity)?;
        if gate.config.schema_version != 4 || prepared.request.root_id != gate.store.root_id {
            return Err(CampaignError::Contract(
                "remediation root or profile differs".into(),
            ));
        }
        let digest = campaign_digest(&json!({"prepared":prepared,"operationId":operation_id,
            "command":command}))?;
        if let Some((receipt, replayed)) = gate.replay(
            operation_id,
            "authorize_remediation_dispatch",
            &digest,
            &prepared.request.identity,
        )? {
            let record = remediation_round(&replayed, &prepared.request.round_id)?
                .dispatch
                .clone()
                .filter(|d| d.operation_id == operation_id)
                .ok_or_else(|| {
                    CampaignError::Integrity("replayed remediation dispatch absent".into())
                })?;
            return Ok(RemediationDispatchEffect::Replayed { record, receipt });
        }
        let state = gate
            .store
            .read(&prepared.request.identity)?
            .ok_or_else(|| CampaignError::Conflict("campaign absent".into()))?;
        let progress = state
            .remediation
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("remediation absent".into()))?;
        let round = progress
            .rounds
            .last()
            .ok_or_else(|| CampaignError::Integrity("round absent".into()))?;
        if command != round.frozen_dispatch_command {
            return Err(CampaignError::Contract(
                "dispatch differs from frozen command".into(),
            ));
        }
        if state.revision != prepared.request.prepared_revision
            || round.request.request_digest != prepared.request.request_digest
            || round.request.round_id != prepared.request.round_id
            || round.dispatch.is_some()
            || round.outcome.is_some()
            || gate
                .store
                .slot(&state.identity, &round.request.obligation_id)?
                != Some((
                    round.request.operation_id.clone(),
                    round.request.request_digest.clone(),
                    round.request.prepared_revision.as_str().into(),
                ))
        {
            return Ok(RemediationDispatchEffect::NoEffect(
                "prepared round is not current".into(),
            ));
        }
        let receipt = gate
            .store
            .receipt(&prepared.request.operation_id)?
            .ok_or_else(|| CampaignError::Integrity("preparation receipt absent".into()))?;
        if encode(&gate.locator(
            &state.identity,
            &prepared.request.operation_id,
            "prepare_remediation",
            receipt.request_digest.clone(),
            receipt.prior_revision.clone(),
        ))? != encode(&prepared.original_registration)?
        {
            return Err(CampaignError::Contract("prepared locator differs".into()));
        }
        let record = RemediationDispatchRecord {
            operation_id: operation_id.into(),
            request_digest: round.request.request_digest.clone(),
            command,
            may_have_entered: true,
        };
        let mut next = state.clone();
        next.remediation
            .as_mut()
            .unwrap()
            .rounds
            .last_mut()
            .unwrap()
            .dispatch = Some(record.clone());
        next.revision = state_revision(&next)?;
        let locator = gate.locator(
            &next.identity,
            operation_id,
            "authorize_remediation_dispatch",
            digest.clone(),
            Some(state.revision.clone()),
        );
        let receipt = gate.receipt(
            &next,
            Some(state.revision.clone()),
            operation_id,
            "authorize_remediation_dispatch",
            digest,
        );
        match gate
            .store
            .write(&next, Some(&state.revision), &receipt, false, None)
        {
            Ok(()) => Ok(RemediationDispatchEffect::Applied {
                permit: RemediationDispatchPermit {
                    owner_epoch: gate.store.epoch.clone(),
                    root_id: gate.store.root_id.clone(),
                    identity: next.identity,
                    round_id: prepared.request.round_id.clone(),
                    request_digest: prepared.request.request_digest.clone(),
                    operation_id: operation_id.into(),
                    dispatched_revision: next.revision,
                },
                receipt,
            }),
            Err(CampaignError::Conflict(e) | CampaignError::Contract(e)) => {
                Ok(RemediationDispatchEffect::NoEffect(e))
            }
            Err(_) => Ok(RemediationDispatchEffect::OutcomeUnknown(locator)),
        }
    }

    pub fn consume_remediation_dispatch_permit(
        &self,
        permit: RemediationDispatchPermit,
    ) -> Result<DispatchedRemediation> {
        let gate = self.owner()?;
        if gate.store.epoch != permit.owner_epoch || gate.store.root_id != permit.root_id {
            return Err(CampaignError::Conflict(
                "remediation permit is stale".into(),
            ));
        }
        let state = gate
            .store
            .read(&permit.identity)?
            .ok_or_else(|| CampaignError::Conflict("campaign absent".into()))?;
        let round = remediation_round(&state, &permit.round_id)?;
        let dispatch = round
            .dispatch
            .as_ref()
            .ok_or_else(|| CampaignError::Integrity("remediation dispatch absent".into()))?;
        if state.revision != permit.dispatched_revision
            || round.request.request_digest != permit.request_digest
            || dispatch.operation_id != permit.operation_id
            || gate
                .store
                .dispatch_slot(&state.identity, &round.request.obligation_id)?
                != Some((Some(permit.operation_id), true))
        {
            return Err(CampaignError::Conflict(
                "remediation permit no longer current".into(),
            ));
        }
        Ok(DispatchedRemediation {
            request: round.request.clone(),
            command: dispatch.command.clone(),
        })
    }
}

fn checked_initial_outcome_manifest(
    state: &Snapshot,
    obligation_id: &str,
    locator: &InitialOutcomeLocator,
) -> Result<OutcomeManifest> {
    if locator.joined.identity != state.identity || locator.joined.obligation_id != obligation_id {
        return Err(CampaignError::Contract(
            "initial outcome locator differs".into(),
        ));
    }
    let intent = state
        .progress
        .attempts
        .iter()
        .filter_map(|a| a.completion_intent.as_ref())
        .find(|i| i.obligation_id == obligation_id)
        .ok_or_else(|| CampaignError::Integrity("initial completion intent absent".into()))?;
    let saved = stage_by_kind(intent, StageKind::InitialOutcome)
        .ok_or_else(|| CampaignError::Integrity("initial outcome registration absent".into()))?;
    let J2Command::Initial(original) = decode(&saved.command)? else {
        return Err(CampaignError::Integrity(
            "initial outcome command differs".into(),
        ));
    };
    if original.operation_id != locator.outcome_operation_id || intent.outcome.is_none() {
        return Err(CampaignError::Contract(
            "initial outcome is not settled".into(),
        ));
    }
    Ok(original)
}

pub(crate) fn validate_remediation_progress(
    store: &Store,
    state: &Snapshot,
    historical: bool,
) -> Result<()> {
    let fail = || CampaignError::Integrity("remediation lineage, receipt or slot differs".into());
    if state.schema_version == 3 {
        return if state.remediation.is_none() {
            Ok(())
        } else {
            Err(fail())
        };
    }
    let Some(progress) = &state.remediation else {
        return Ok(());
    };
    if progress.current_generation as usize != progress.rounds.len() || progress.rounds.is_empty() {
        return Err(fail());
    }
    let initial = state.candidate.as_ref().ok_or_else(fail)?;
    let original_initial = progress
        .rounds
        .first()
        .map(|r| &r.initial_outcome)
        .ok_or_else(fail)?;
    let initial_intent = state
        .progress
        .attempts
        .iter()
        .filter_map(|attempt| attempt.completion_intent.as_ref())
        .find(|intent| intent.obligation_id == original_initial.joined.obligation_id)
        .ok_or_else(fail)?;
    let mut prior = initial;
    let mut identities = std::collections::BTreeSet::new();
    for (index, round) in progress.rounds.iter().enumerate() {
        let request = &round.request;
        let generation = (index + 1) as u64;
        if request.generation != generation
            || round.predecessor_generation != generation - 1
            || request.round_id != format!("generation-{generation}")
            || request.identity != state.identity
            || request.root_id != store.root_id
            || round.initial_outcome.outcome_operation_id != original_initial.outcome_operation_id
            || round.initial_outcome.joined.identity != state.identity
            || round.initial_outcome.joined.obligation_id != request.obligation_id
            || !identities.insert(request.operation_id.clone())
            || !identities.insert(request.execution_attempt_id.clone())
            || round.frozen_dispatch_command.validate(request).is_err()
            || round.acceptance_sources.is_empty()
            || round
                .acceptance_sources
                .iter()
                .any(|s| s["status"] != "verified")
        {
            return Err(fail());
        }
        if index > 0 && progress.rounds[index - 1].outcome.is_none() {
            return Err(fail());
        }
        if request.session.owner != initial_intent.execution_evidence_ref.owner
            || request.session.root_id != initial_intent.execution_evidence_ref.root_id
            || request.session.initial_attempt_id != initial_intent.attempt_id
            || request.session.initial_attempt_id
                != initial_intent.execution_evidence_ref.attempt_id
            || initial_intent.execution_session_id.as_deref()
                != Some(request.session.session_id.as_str())
            || (index > 0 && request.session != progress.rounds[0].request.session)
        {
            return Err(fail());
        }
        if index > 0 {
            let predecessor = &progress.rounds[index - 1];
            let outcome = predecessor.outcome.as_ref().ok_or_else(fail)?;
            let prior_result = outcome["nativeResultCanonicalJson"]
                .as_str()
                .ok_or_else(fail)?;
            let prior_count = outcome["nativeFindingCount"].as_u64().ok_or_else(fail)?;
            if round.predecessor_result_canonical_json != prior_result
                || prior_count != round.predecessor_finding_refs.len() as u64
                || (outcome["kind"] != "awaiting_builder"
                    && !(outcome["kind"] == "reported" && prior_count == 0))
                || predecessor
                    .evaluations
                    .iter()
                    .any(|evaluation| evaluation.publication_state != "committed")
            {
                return Err(fail());
            }
        }
        let mut baseline = state.baseline.clone();
        baseline.inter_slice_commit = prior.commit.clone();
        let verified = crate::subject_binding::verify_existing_subject_dynamic(
            std::path::Path::new(&state.repository),
            &state.identity,
            &baseline,
            round.candidate_receipt_raw.as_bytes(),
            round.physical_profile_raw.as_bytes(),
            round.gate_capture_raw.as_bytes(),
            round.gate_manifest_raw.as_bytes(),
            round.gate_receipt_raw.as_bytes(),
        )
        .map_err(|_| fail())?;
        let selected = validate_selection(
            &round.accepted_selection,
            &state.identity,
            &verified.reference,
        )
        .map_err(|_| fail())?;
        if request.candidate != verified.reference
            || request.selection != selected
            || request.profile_digest != verified.profile_digest
            || request.candidate.tree == prior.tree
            || round.accepted_claims.len() != 2
            || round.accepted_claims.iter().any(|claim| {
                round.accepted_selection["specialists"]
                    .as_array()
                    .is_none_or(|items| {
                        !items.iter().any(|item| {
                            item["obligationId"] == request.obligation_id
                                && item["requiredClaims"].as_array().is_some_and(|claims| {
                                    claims.iter().any(|c| {
                                        c["claimId"] == claim.claim_id
                                            && c["revision"] == claim.revision
                                            && c["profile"]["continuity"] == "retained"
                                    })
                                })
                        })
                    })
            })
            || request.session.validate().is_err()
            || request.session.initial_attempt_id != state.identity.attempt_id
        {
            return Err(fail());
        }
        let expected_digest = campaign_digest(&json!({"rootId":request.root_id,"identity":request.identity,
            "obligationId":request.obligation_id,"roundId":request.round_id,"generation":request.generation,
            "operationId":request.operation_id,"executionAttemptId":request.execution_attempt_id,
            "candidate":request.candidate,"selection":request.selection,"profileDigest":request.profile_digest,
            "session":request.session,"dispatchCommand":round.frozen_dispatch_command})).map_err(|_| fail())?;
        if request.request_digest != expected_digest {
            return Err(fail());
        }
        let slot = store
            .slot_for_operation(&state.identity, &request.operation_id)?
            .ok_or_else(fail)?;
        if slot.obligation_id != request.obligation_id
            || slot.attempt_id != request.execution_attempt_id
            || slot.request_digest != request.request_digest
            || slot.prepared_revision != request.prepared_revision.as_str()
            || (!historical && slot.active != (index + 1 == progress.rounds.len()))
        {
            return Err(fail());
        }
        let (preparation_receipt, preparation_state) = store
            .projection_result(&request.operation_id, &state.identity)?
            .ok_or_else(fail)?;
        let original = preparation_state
            .remediation
            .as_ref()
            .and_then(|p| p.rounds.get(index))
            .ok_or_else(fail)?;
        let mut current_immutable = serde_json::to_value(round).map_err(|_| fail())?;
        let mut original_immutable = serde_json::to_value(original).map_err(|_| fail())?;
        for value in [&mut current_immutable, &mut original_immutable] {
            value.as_object_mut().ok_or_else(fail)?.remove("dispatch");
            value
                .as_object_mut()
                .ok_or_else(fail)?
                .remove("subjectRead");
            value.as_object_mut().ok_or_else(fail)?.remove("completion");
            value.as_object_mut().ok_or_else(fail)?.remove("outcome");
            value
                .as_object_mut()
                .ok_or_else(fail)?
                .remove("evaluations");
        }
        if preparation_receipt.kind != "prepare_remediation"
            || preparation_receipt.result_revision != request.prepared_revision
            || original_immutable != current_immutable
            || original.dispatch.is_some()
            || original.subject_read.is_some()
            || original.completion.is_some()
            || original.outcome.is_some()
            || !original.evaluations.is_empty()
            || preparation_state.remediation.as_ref().is_none_or(|p| {
                p.current_generation != generation || p.rounds.len() != generation as usize
            })
        {
            return Err(fail());
        }
        match &round.dispatch {
            None if !historical
                && (slot.dispatch_operation_id.is_some() || slot.may_have_entered) =>
            {
                return Err(fail());
            }
            None => {}
            Some(dispatch) => {
                if !dispatch.may_have_entered
                    || dispatch.request_digest != request.request_digest
                    || dispatch.command != round.frozen_dispatch_command
                    || slot.dispatch_operation_id.as_deref() != Some(dispatch.operation_id.as_str())
                    || !slot.may_have_entered
                    || !identities.insert(dispatch.operation_id.clone())
                {
                    return Err(fail());
                }
                let locator = RecoveryLocator {
                    root_id: store.root_id.clone(),
                    anchored_root: store.root.clone(),
                    identity: state.identity.clone(),
                    operation_id: request.operation_id.clone(),
                    kind: "prepare_remediation".into(),
                    request_digest: preparation_receipt.request_digest.clone(),
                    expected_revision: preparation_receipt.prior_revision.clone(),
                    profile_digest: preparation_receipt.profile_digest.clone(),
                };
                let prepared = RemediationPrepared {
                    request: request.clone(),
                    original_registration: locator,
                };
                let expected = campaign_digest(
                    &json!({"prepared":prepared,"operationId":dispatch.operation_id,
                    "command":dispatch.command}),
                )
                .map_err(|_| fail())?;
                let (receipt, dispatched) = store
                    .projection_result(&dispatch.operation_id, &state.identity)?
                    .ok_or_else(fail)?;
                if receipt.kind != "authorize_remediation_dispatch"
                    || receipt.request_digest != expected
                    || dispatched
                        .remediation
                        .as_ref()
                        .and_then(|p| p.rounds.get(index))
                        .and_then(|r| r.dispatch.as_ref())
                        .is_none_or(|saved| {
                            serde_json::to_value(saved).ok() != serde_json::to_value(dispatch).ok()
                        })
                {
                    return Err(fail());
                }
            }
        }
        if let Some(read) = &round.subject_read {
            let dispatch = round.dispatch.as_ref().ok_or_else(fail)?;
            let authority = state
                .progress
                .attempts
                .iter()
                .filter_map(|a| a.dispatch.as_ref())
                .find(|d| d.command.episode_id == dispatch.command.episode_id)
                .ok_or_else(fail)?;
            if read.transition_id_canonical_json
                != dispatch.command.subject_transition_id_canonical_json
                || read.authority_digest.len() != 64
                || read.resolved_revision.len() != 64
                || read.result_digest
                    != episode_value_digest(
                        &parse_episode_json(&round.predecessor_result_canonical_json)
                            .map_err(|_| fail())?,
                    )
                || authority.command.writer_actor != dispatch.command.writer_actor
                || !identities.insert(read.operation_id.clone())
            {
                return Err(fail());
            }
            let prepared = RemediationPrepared {
                request: request.clone(),
                original_registration: RecoveryLocator {
                    root_id: store.root_id.clone(),
                    anchored_root: store.root.clone(),
                    identity: state.identity.clone(),
                    operation_id: request.operation_id.clone(),
                    kind: "prepare_remediation".into(),
                    request_digest: preparation_receipt.request_digest.clone(),
                    expected_revision: preparation_receipt.prior_revision.clone(),
                    profile_digest: preparation_receipt.profile_digest.clone(),
                },
            };
            let (receipt, confirmed) = store
                .projection_result(&read.operation_id, &state.identity)?
                .ok_or_else(fail)?;
            let original = confirmed
                .remediation
                .as_ref()
                .and_then(|p| p.rounds.get(index))
                .and_then(|r| r.subject_read.as_ref())
                .ok_or_else(fail)?;
            let expected = campaign_digest(&json!({"prepared":prepared,
                "expectedRevision":receipt.prior_revision,"subjectRead":original}))
            .map_err(|_| fail())?;
            if receipt.kind != "confirm_remediation_subject_read"
                || receipt.request_digest != expected
                || serde_json::to_value(original).ok() != serde_json::to_value(read).ok()
                || confirmed
                    .remediation
                    .as_ref()
                    .and_then(|p| p.rounds.get(index))
                    .and_then(|r| r.dispatch.as_ref())
                    .is_none_or(|saved| {
                        serde_json::to_value(saved).ok() != serde_json::to_value(dispatch).ok()
                    })
            {
                return Err(fail());
            }
        }
        if let Some(intent) = &round.completion {
            let dispatch = round.dispatch.as_ref().ok_or_else(fail)?;
            if round.subject_read.is_none()
                || intent.campaign_operation_id != intent.registration_operation_id
                || intent.obligation_id != request.obligation_id
                || intent.attempt_id != request.execution_attempt_id
                || intent.preparation_operation_id != request.operation_id
                || intent.prepared_request_sha256 != request.request_digest
                || intent.dispatch_operation_id != dispatch.operation_id
                || intent.selected_claims.len() != 2
                || serde_json::to_value(&intent.selected_claims).ok()
                    != serde_json::to_value(&round.accepted_claims).ok()
                || intent.execution_session_id.as_deref()
                    != Some(request.session.session_id.as_str())
                || intent.execution_evidence_ref.checked().is_err()
                || !identities.insert(intent.campaign_operation_id.clone())
                || crate::require_sha(&intent.native_result_claim_sha256, "result").is_err()
                || crate::require_sha(&intent.native_result_raw_sha256, "raw result").is_err()
            {
                return Err(fail());
            }
            let (receipt, registered) = store
                .projection_result(&intent.registration_operation_id, &state.identity)?
                .ok_or_else(fail)?;
            let original_intent = registered
                .remediation
                .as_ref()
                .and_then(|p| p.rounds.get(index))
                .and_then(|r| r.completion.as_ref())
                .ok_or_else(fail)?;
            let mut current_immutable = serde_json::to_value(intent).map_err(|_| fail())?;
            let mut original_immutable =
                serde_json::to_value(original_intent).map_err(|_| fail())?;
            for value in [&mut current_immutable, &mut original_immutable] {
                let fields = value.as_object_mut().ok_or_else(fail)?;
                fields.remove("stages");
                fields.remove("consumptionIntents");
                fields.remove("joinedReadbacks");
                fields.remove("outcome");
            }
            let prepared = RemediationPrepared {
                request: request.clone(),
                original_registration: RecoveryLocator {
                    root_id: store.root_id.clone(),
                    anchored_root: store.root.clone(),
                    identity: state.identity.clone(),
                    operation_id: request.operation_id.clone(),
                    kind: "prepare_remediation".into(),
                    request_digest: preparation_receipt.request_digest.clone(),
                    expected_revision: preparation_receipt.prior_revision.clone(),
                    profile_digest: preparation_receipt.profile_digest.clone(),
                },
            };
            let expected = campaign_digest(&json!({"prepared":prepared,
                "expectedRevision":intent.registration_base_revision,
                "operationId":intent.registration_operation_id,"intent":original_intent}))
            .map_err(|_| fail())?;
            if receipt.kind != "prepare_remediation_completion"
                || receipt.prior_revision.as_ref() != Some(&intent.registration_base_revision)
                || receipt.request_digest != expected
                || original_immutable != current_immutable
                || !original_intent.stages.is_empty()
                || !original_intent.consumption_intents.is_empty()
                || original_intent.joined_readbacks.is_some()
                || original_intent.outcome.is_some()
                || registered
                    .remediation
                    .as_ref()
                    .and_then(|p| p.rounds.get(index))
                    .and_then(|r| r.dispatch.as_ref())
                    .is_none_or(|saved| {
                        serde_json::to_value(saved).ok() != serde_json::to_value(dispatch).ok()
                    })
            {
                return Err(fail());
            }
            let completion_locator = RemediationCompletionPrepared {
                round: request.clone(),
                completion_operation_id: intent.campaign_operation_id.clone(),
                original_registration: RecoveryLocator {
                    root_id: store.root_id.clone(),
                    anchored_root: store.root.clone(),
                    identity: state.identity.clone(),
                    operation_id: intent.campaign_operation_id.clone(),
                    kind: "prepare_remediation_completion".into(),
                    request_digest: receipt.request_digest.clone(),
                    expected_revision: receipt.prior_revision.clone(),
                    profile_digest: receipt.profile_digest.clone(),
                },
            };
            let mut stage_ids = std::collections::BTreeSet::new();
            let mut kinds = std::collections::BTreeSet::new();
            for (stage_index, stage) in intent.stages.iter().enumerate() {
                if matches!(
                    stage.kind,
                    StageKind::EvaluationReliance | StageKind::EvaluationProjection
                ) {
                    if !stage_ids.insert(stage.stage_id.clone())
                        || stage.owner_selection_digest != intent.owner_selection_digest
                        || stage.evidence_ref != intent.execution_evidence_ref
                    {
                        return Err(fail());
                    }
                    let record = round
                        .evaluations
                        .iter()
                        .find(|record| record.operation_id == stage.registration_operation_id)
                        .ok_or_else(fail)?;
                    let (receipt, registered) = store
                        .projection_result(&record.operation_id, &state.identity)?
                        .ok_or_else(fail)?;
                    let original_round = registered
                        .remediation
                        .as_ref()
                        .and_then(|p| p.rounds.get(index))
                        .ok_or_else(fail)?;
                    let original_record = original_round
                        .evaluations
                        .iter()
                        .find(|r| r.operation_id == record.operation_id)
                        .ok_or_else(fail)?;
                    let original_intent = original_round.completion.as_ref().ok_or_else(fail)?;
                    let original_stage =
                        original_intent.stages.get(stage_index).ok_or_else(fail)?;
                    let reliance_stage = original_intent
                        .stages
                        .iter()
                        .find(|s| {
                            s.kind == StageKind::EvaluationReliance
                                && s.registration_operation_id == record.operation_id
                        })
                        .ok_or_else(fail)?;
                    let projection_stage = original_intent
                        .stages
                        .iter()
                        .find(|s| {
                            s.kind == StageKind::EvaluationProjection
                                && s.registration_operation_id == record.operation_id
                        })
                        .ok_or_else(fail)?;
                    let expected = campaign_digest(&json!({"completion":completion_locator,
                        "expectedRevision":stage.registration_base_revision,
                        "record":original_record,"stage":reliance_stage,
                        "projectionStage":projection_stage}))
                    .map_err(|_| fail())?;
                    let mut immutable = serde_json::to_value(stage).map_err(|_| fail())?;
                    let mut original = serde_json::to_value(original_stage).map_err(|_| fail())?;
                    immutable
                        .as_object_mut()
                        .ok_or_else(fail)?
                        .remove("progress");
                    original
                        .as_object_mut()
                        .ok_or_else(fail)?
                        .remove("progress");
                    if receipt.kind != "prepare_remediation_evaluation"
                        || receipt.prior_revision.as_ref()
                            != Some(&stage.registration_base_revision)
                        || receipt.request_digest != expected
                        || immutable != original
                        || !matches!(original_stage.progress, StageProgress::Registered)
                        || original_record.publication_state != "registered"
                    {
                        return Err(fail());
                    }
                    let admission = binding_for(stage).map_err(|_| fail())?;
                    if admission.campaign_revision != stage.registration_base_revision.as_str()
                        || admission.prepared_request_sha256 != request.request_digest
                        || admission.candidate_digest != request.candidate.receipt_sha256
                        || admission.result_sha256 != intent.native_result_claim_sha256
                        || admission.digest() != stage.original_admission_sha256
                    {
                        return Err(fail());
                    }
                    let expected_request = if stage.kind == StageKind::EvaluationReliance {
                        let J2Command::Evaluation(envelope) =
                            decode(&stage.command).map_err(|_| fail())?
                        else {
                            return Err(fail());
                        };
                        if stage.stage_id != format!("{}:reliance", record.operation_id)
                            || envelope.claim_id != record.finding_id
                            || envelope.finding_revision != record.finding_revision
                            || envelope.valid != record.valid
                        {
                            return Err(fail());
                        }
                        claim_evidence::reliance_request_sha256(
                            &reliance_command(&envelope, admission).map_err(|_| fail())?,
                        )
                        .map_err(|_| fail())?
                    } else {
                        let J2Command::Projection(envelope) =
                            decode(&stage.command).map_err(|_| fail())?
                        else {
                            return Err(fail());
                        };
                        if stage.stage_id != format!("{}:projection", record.operation_id)
                            || envelope.consumer_tree != record.consumer_tree
                            || envelope.decision_scope != record.decision_scope
                        {
                            return Err(fail());
                        }
                        claim_evidence::projection_request_sha256(
                            &projection_command(&envelope, admission).map_err(|_| fail())?,
                        )
                        .map_err(|_| fail())?
                    };
                    if stage.request_sha256 != expected_request {
                        return Err(fail());
                    }
                    match &stage.progress {
                        StageProgress::Registered => {}
                        StageProgress::Checked {
                            exact_refs,
                            content_digests,
                        } if exact_refs.len() == 1
                            && content_digests.len() == 1
                            && campaign_digest(&exact_refs[0]).ok().as_deref()
                                == Some(content_digests[0].as_str()) => {}
                        _ => return Err(fail()),
                    }
                    if !matches!(stage.progress, StageProgress::Registered) {
                        let progress_id = format!("{}:progress", stage.stage_id);
                        if !identities.insert(progress_id.clone()) {
                            return Err(fail());
                        }
                        let (progress_receipt, committed) = store
                            .projection_result(&progress_id, &state.identity)?
                            .ok_or_else(fail)?;
                        let saved = committed
                            .remediation
                            .as_ref()
                            .and_then(|p| p.rounds.get(index))
                            .and_then(|r| r.completion.as_ref())
                            .and_then(|i| i.stages.get(stage_index))
                            .map(|s| &s.progress)
                            .ok_or_else(fail)?;
                        let expected = campaign_digest(&json!({"completion":completion_locator,
                            "stageId":stage.stage_id,"progress":stage.progress}))
                        .map_err(|_| fail())?;
                        if progress_receipt.kind != "record_remediation_stage"
                            || progress_receipt.request_digest != expected
                            || serde_json::to_value(saved).ok()
                                != serde_json::to_value(&stage.progress).ok()
                        {
                            return Err(fail());
                        }
                    }
                    continue;
                }
                if matches!(
                    stage.kind,
                    StageKind::InitialOutcome | StageKind::Finding | StageKind::FindingProjection
                ) {
                    let registration_kind = match stage.kind {
                        StageKind::InitialOutcome => "register_remediation_outcome",
                        StageKind::Finding => "register_remediation_finding",
                        StageKind::FindingProjection => "register_remediation_projection",
                        _ => unreachable!(),
                    };
                    if !stage_ids.insert(stage.stage_id.clone())
                        || !identities.insert(stage.registration_operation_id.clone())
                        || stage.owner_selection_digest != intent.owner_selection_digest
                        || stage.evidence_ref != intent.execution_evidence_ref
                    {
                        return Err(fail());
                    }
                    let (receipt, registered) = store
                        .projection_result(&stage.registration_operation_id, &state.identity)?
                        .ok_or_else(fail)?;
                    let original = registered
                        .remediation
                        .as_ref()
                        .and_then(|p| p.rounds.get(index))
                        .and_then(|r| r.completion.as_ref())
                        .and_then(|i| i.stages.get(stage_index))
                        .ok_or_else(fail)?;
                    let mut mutable = serde_json::to_value(stage).map_err(|_| fail())?;
                    let mut immutable = serde_json::to_value(original).map_err(|_| fail())?;
                    mutable.as_object_mut().ok_or_else(fail)?.remove("progress");
                    immutable
                        .as_object_mut()
                        .ok_or_else(fail)?
                        .remove("progress");
                    let expected = campaign_digest(&json!({"completion":completion_locator,
                        "stage":original}))
                    .map_err(|_| fail())?;
                    if receipt.kind != registration_kind
                        || receipt.prior_revision.as_ref()
                            != Some(&stage.registration_base_revision)
                        || receipt.request_digest != expected
                        || mutable != immutable
                        || !matches!(original.progress, StageProgress::Registered)
                    {
                        return Err(fail());
                    }
                    match stage.kind {
                        StageKind::InitialOutcome => {
                            let manifest: RemediationOutcomeManifest =
                                decode(&stage.command).map_err(|_| fail())?;
                            let result = parse_episode_json(&manifest.native_result_canonical_json)
                                .map_err(|_| fail())?;
                            ReviewResult::parse(result).map_err(|_| fail())?;
                            if !kinds.insert(stage.kind)
                                || stage.stage_id != format!("{}:outcome", manifest.operation_id)
                                || stage.registration_operation_id != manifest.operation_id
                                || stage.request_sha256
                                    != campaign_digest(&stage.command).map_err(|_| fail())?
                                || manifest.finding_commands.len() > 100
                                || !matches!(stage.progress, StageProgress::Registered)
                            {
                                return Err(fail());
                            }
                        }
                        StageKind::Finding => {
                            let saved: RemediationFindingCommand =
                                decode(&stage.command).map_err(|_| fail())?;
                            let admission = binding_for(stage).map_err(|_| fail())?;
                            let command = remediation_finding_command(&saved, admission.clone())
                                .map_err(|_| fail())?;
                            if stage.stage_id != saved.envelope.operation_id
                                || stage.registration_operation_id
                                    != format!("{}:register", saved.envelope.operation_id)
                                || admission.campaign_revision
                                    != stage.registration_base_revision.as_str()
                                || admission.prepared_request_sha256 != request.request_digest
                                || admission.candidate_digest != request.candidate.receipt_sha256
                                || admission.result_sha256 != intent.native_result_claim_sha256
                                || stage.request_sha256
                                    != claim_evidence::claims_request_sha256(&command)
                                        .map_err(|_| fail())?
                                || stage.original_admission_sha256 != admission.digest()
                                || stage.owner_locator.is_null()
                            {
                                return Err(fail());
                            }
                        }
                        StageKind::FindingProjection => {
                            let J2Command::Projection(envelope) =
                                decode(&stage.command).map_err(|_| fail())?
                            else {
                                return Err(fail());
                            };
                            let admission = binding_for(stage).map_err(|_| fail())?;
                            let command = projection_command(&envelope, admission.clone())
                                .map_err(|_| fail())?;
                            if !kinds.insert(stage.kind)
                                || stage.stage_id != envelope.request_id
                                || stage.registration_operation_id
                                    != format!("{}:register", envelope.request_id)
                                || admission.campaign_revision
                                    != stage.registration_base_revision.as_str()
                                || admission.prepared_request_sha256 != request.request_digest
                                || admission.candidate_digest != request.candidate.receipt_sha256
                                || admission.result_sha256 != intent.native_result_claim_sha256
                                || stage.request_sha256
                                    != claim_evidence::projection_request_sha256(&command)
                                        .map_err(|_| fail())?
                                || stage.original_admission_sha256 != admission.digest()
                            {
                                return Err(fail());
                            }
                        }
                        _ => unreachable!(),
                    }
                    if stage.kind != StageKind::InitialOutcome {
                        match &stage.progress {
                            StageProgress::Registered => {}
                            StageProgress::Checked {
                                exact_refs,
                                content_digests,
                            } if exact_refs.len() == 1
                                && content_digests.len() == 1
                                && campaign_digest(&exact_refs[0]).ok().as_deref()
                                    == Some(content_digests[0].as_str()) => {}
                            _ => return Err(fail()),
                        }
                        if !matches!(stage.progress, StageProgress::Registered) {
                            let progress_id = format!("{}:progress", stage.stage_id);
                            if !identities.insert(progress_id.clone()) {
                                return Err(fail());
                            }
                            let (progress_receipt, committed) = store
                                .projection_result(&progress_id, &state.identity)?
                                .ok_or_else(fail)?;
                            let saved_progress = committed
                                .remediation
                                .as_ref()
                                .and_then(|p| p.rounds.get(index))
                                .and_then(|r| r.completion.as_ref())
                                .and_then(|i| i.stages.get(stage_index))
                                .map(|s| &s.progress)
                                .ok_or_else(fail)?;
                            let expected =
                                campaign_digest(&json!({"completion":completion_locator,
                                "stageId":stage.stage_id,"progress":stage.progress}))
                                .map_err(|_| fail())?;
                            if progress_receipt.kind != "record_remediation_stage"
                                || progress_receipt.request_digest != expected
                                || serde_json::to_value(saved_progress).ok()
                                    != serde_json::to_value(&stage.progress).ok()
                            {
                                return Err(fail());
                            }
                        }
                    }
                    continue;
                }
                if stage.kind == StageKind::EpisodeResultRead {
                    let content = parse_episode_json(
                        stage.command["contentCanonicalJson"]
                            .as_str()
                            .ok_or_else(fail)?,
                    )
                    .map_err(|_| fail())?;
                    let digest = episode_value_digest(&content);
                    if !kinds.insert(stage.kind)
                        || !stage_ids.insert(stage.stage_id.clone())
                        || !identities.insert(stage.registration_operation_id.clone())
                        || stage.stage_id != stage_id(&intent.campaign_operation_id, stage.kind)
                        || stage.owner_selection_digest != intent.owner_selection_digest
                        || stage.evidence_ref != intent.execution_evidence_ref
                        || stage.request_sha256 != digest
                        || stage.original_admission_sha256 != digest
                        || stage.original_admission != Value::Null
                        || stage.command["contentDigest"] != digest
                        || stage.owner_locator["contentDigest"] != digest
                        || stage.owner_locator["transitionIdCanonicalJson"]
                            != round
                                .frozen_dispatch_command
                                .result_transition_id_canonical_json
                        || intent.consumption_intents.len() != 2
                    {
                        return Err(fail());
                    }
                    let (stage_receipt, registered) = store
                        .projection_result(&stage.registration_operation_id, &state.identity)?
                        .ok_or_else(fail)?;
                    let registered_intent = registered
                        .remediation
                        .as_ref()
                        .and_then(|p| p.rounds.get(index))
                        .and_then(|r| r.completion.as_ref())
                        .ok_or_else(fail)?;
                    let original_stage =
                        registered_intent.stages.get(stage_index).ok_or_else(fail)?;
                    let expected = campaign_digest(&json!({"completion":completion_locator,
                        "expectedRevision":stage.registration_base_revision,
                        "stage":original_stage,
                        "consumptions":registered_intent.consumption_intents}))
                    .map_err(|_| fail())?;
                    let mut current_stage = serde_json::to_value(stage).map_err(|_| fail())?;
                    let mut registered_stage =
                        serde_json::to_value(original_stage).map_err(|_| fail())?;
                    current_stage
                        .as_object_mut()
                        .ok_or_else(fail)?
                        .remove("progress");
                    registered_stage
                        .as_object_mut()
                        .ok_or_else(fail)?
                        .remove("progress");
                    let mut registered_consumptions =
                        serde_json::to_value(&registered_intent.consumption_intents)
                            .map_err(|_| fail())?;
                    let mut current_consumptions =
                        serde_json::to_value(&intent.consumption_intents).map_err(|_| fail())?;
                    for group in [&mut registered_consumptions, &mut current_consumptions] {
                        for item in group.as_array_mut().ok_or_else(fail)? {
                            item.as_object_mut()
                                .ok_or_else(fail)?
                                .remove("consumedAtOperationId");
                        }
                    }
                    if stage_receipt.kind != "register_remediation_episode_read"
                        || stage_receipt.prior_revision.as_ref()
                            != Some(&stage.registration_base_revision)
                        || stage_receipt.request_digest != expected
                        || current_stage != registered_stage
                        || current_consumptions != registered_consumptions
                        || registered_intent
                            .consumption_intents
                            .iter()
                            .any(|c| c.consumed_at_operation_id.is_some())
                        || !matches!(original_stage.progress, StageProgress::Registered)
                    {
                        return Err(fail());
                    }
                    match &stage.progress {
                        StageProgress::Registered => {}
                        StageProgress::Checked {
                            exact_refs,
                            content_digests,
                        } if exact_refs.len() == 1
                            && content_digests.len() == 1
                            && campaign_digest(&exact_refs[0]).ok().as_deref()
                                == Some(content_digests[0].as_str()) => {}
                        _ => return Err(fail()),
                    }
                    if !matches!(stage.progress, StageProgress::Registered) {
                        let progress_id = format!("{}:progress", stage.stage_id);
                        if !identities.insert(progress_id.clone()) {
                            return Err(fail());
                        }
                        let (receipt, committed) = store
                            .projection_result(&progress_id, &state.identity)?
                            .ok_or_else(fail)?;
                        let saved = committed
                            .remediation
                            .as_ref()
                            .and_then(|p| p.rounds.get(index))
                            .and_then(|r| r.completion.as_ref())
                            .and_then(|i| i.stages.get(stage_index))
                            .map(|s| &s.progress)
                            .ok_or_else(fail)?;
                        let expected = campaign_digest(&json!({"completion":completion_locator,
                            "stageId":stage.stage_id,"progress":stage.progress}))
                        .map_err(|_| fail())?;
                        if receipt.kind != "record_remediation_stage"
                            || receipt.request_digest != expected
                            || serde_json::to_value(saved).ok()
                                != serde_json::to_value(&stage.progress).ok()
                        {
                            return Err(fail());
                        }
                    }
                    continue;
                }
                if !matches!(
                    stage.kind,
                    StageKind::Observation
                        | StageKind::EstablishmentBuilder
                        | StageKind::EstablishmentCampaign
                        | StageKind::AdmissionBuilder
                        | StageKind::AdmissionCampaign
                ) || !kinds.insert(stage.kind)
                    || !stage_ids.insert(stage.stage_id.clone())
                    || !identities.insert(stage.registration_operation_id.clone())
                    || stage.stage_id != stage_id(&intent.campaign_operation_id, stage.kind)
                    || stage.registration_operation_id != format!("{}:register", stage.stage_id)
                    || stage.owner_selection_digest != intent.owner_selection_digest
                    || stage.evidence_ref != intent.execution_evidence_ref
                    || stage.owner_locator.is_null()
                {
                    return Err(fail());
                }
                let ce_request =
                    stage_request(stage, ProductionPathAccess::Original).map_err(|_| fail())?;
                let binding = match &ce_request {
                    ProductionPathRequest::RecordObservation(command) => &command.admission,
                    ProductionPathRequest::EstablishClaim(command) => &command.admission,
                    ProductionPathRequest::ReadAdmission(command) => &command.admission,
                };
                if binding.campaign_root_id != request.root_id
                    || binding.campaign_revision != stage.registration_base_revision.as_str()
                    || binding.campaign_operation_id != intent.campaign_operation_id
                    || binding.obligation_id != request.obligation_id
                    || binding.candidate_digest != request.candidate.receipt_sha256
                    || binding.selection_digest != request.selection.campaign_digest
                    || binding.profile_digest != request.profile_digest
                    || binding.prepared_request_sha256 != request.request_digest
                    || binding.review_episode_id != round.frozen_dispatch_command.episode_id
                    || binding.attempt_id != request.execution_attempt_id
                    || binding.native_result_sha256 != intent.native_result_claim_sha256
                    || binding.original_digest() != stage.original_admission_sha256
                    || production_path_request_sha256(&ce_request).ok().as_deref()
                        != Some(stage.request_sha256.as_str())
                {
                    return Err(fail());
                }
                let (stage_receipt, registered) = store
                    .projection_result(&stage.registration_operation_id, &state.identity)?
                    .ok_or_else(fail)?;
                let original_stage = registered
                    .remediation
                    .as_ref()
                    .and_then(|p| p.rounds.get(index))
                    .and_then(|r| r.completion.as_ref())
                    .and_then(|i| i.stages.get(stage_index))
                    .ok_or_else(fail)?;
                let expected = campaign_digest(&json!({"completion":completion_locator,
                    "stage":original_stage,"expectedRevision":stage.registration_base_revision}))
                .map_err(|_| fail())?;
                let mut current_stage = serde_json::to_value(stage).map_err(|_| fail())?;
                let mut registered_stage =
                    serde_json::to_value(original_stage).map_err(|_| fail())?;
                current_stage
                    .as_object_mut()
                    .ok_or_else(fail)?
                    .remove("progress");
                registered_stage
                    .as_object_mut()
                    .ok_or_else(fail)?
                    .remove("progress");
                if stage_receipt.kind != "register_remediation_stage"
                    || stage_receipt.prior_revision.as_ref()
                        != Some(&stage.registration_base_revision)
                    || stage_receipt.request_digest != expected
                    || current_stage != registered_stage
                    || !matches!(original_stage.progress, StageProgress::Registered)
                    || registered
                        .remediation
                        .as_ref()
                        .and_then(|p| p.rounds.get(index))
                        .and_then(|r| r.completion.as_ref())
                        .is_none_or(|i| i.stages.len() != stage_index + 1)
                {
                    return Err(fail());
                }
                match &stage.progress {
                    StageProgress::Registered => {}
                    StageProgress::OwnerCommitted {
                        receipt: owner_receipt,
                    } => {
                        if !matches!(
                            stage.kind,
                            StageKind::Observation
                                | StageKind::EstablishmentBuilder
                                | StageKind::EstablishmentCampaign
                        ) {
                            return Err(fail());
                        }
                        let owner = receipt_from_value(owner_receipt).map_err(|_| fail())?;
                        let mut expected_locator =
                            locator_from_value(&stage.owner_locator).map_err(|_| fail())?;
                        expected_locator.custody_sha256 = owner.locator.custody_sha256.clone();
                        if owner.locator != expected_locator {
                            return Err(fail());
                        }
                    }
                    StageProgress::Checked {
                        exact_refs,
                        content_digests,
                    } => {
                        if !matches!(
                            stage.kind,
                            StageKind::AdmissionBuilder | StageKind::AdmissionCampaign
                        ) || exact_refs.len() != 1
                            || content_digests.len() != 1
                            || campaign_digest(&exact_refs[0]).ok().as_deref()
                                != Some(content_digests[0].as_str())
                        {
                            return Err(fail());
                        }
                    }
                    StageProgress::Conflicting { .. } | StageProgress::Unresolved { .. } => {
                        return Err(fail());
                    }
                }
                if !matches!(stage.progress, StageProgress::Registered) {
                    let progress_id = format!("{}:progress", stage.stage_id);
                    if !identities.insert(progress_id.clone()) {
                        return Err(fail());
                    }
                    let (progress_receipt, committed) = store
                        .projection_result(&progress_id, &state.identity)?
                        .ok_or_else(fail)?;
                    let saved_progress = committed
                        .remediation
                        .as_ref()
                        .and_then(|p| p.rounds.get(index))
                        .and_then(|r| r.completion.as_ref())
                        .and_then(|i| i.stages.get(stage_index))
                        .map(|s| &s.progress)
                        .ok_or_else(fail)?;
                    let expected = campaign_digest(&json!({"completion":completion_locator,
                        "stageId":stage.stage_id,"progress":stage.progress}))
                    .map_err(|_| fail())?;
                    if progress_receipt.kind != "record_remediation_stage"
                        || progress_receipt.request_digest != expected
                        || serde_json::to_value(saved_progress).ok()
                            != serde_json::to_value(&stage.progress).ok()
                    {
                        return Err(fail());
                    }
                }
            }
            if let Some(joined) = &intent.joined_readbacks {
                let builder = checked_record_from_stage(
                    stage_by_kind(intent, StageKind::AdmissionBuilder).ok_or_else(fail)?,
                )
                .map_err(|_| fail())?;
                let terminal = checked_record_from_stage(
                    stage_by_kind(intent, StageKind::AdmissionCampaign).ok_or_else(fail)?,
                )
                .map_err(|_| fail())?;
                let episode = checked_record_from_stage(
                    stage_by_kind(intent, StageKind::EpisodeResultRead).ok_or_else(fail)?,
                )
                .map_err(|_| fail())?;
                if joined.builder != builder
                    || joined.campaign != terminal
                    || joined.episode != episode
                {
                    return Err(fail());
                }
                let operation_id = format!("{}:joined", intent.campaign_operation_id);
                if !identities.insert(operation_id.clone()) {
                    return Err(fail());
                }
                let (receipt, committed) = store
                    .projection_result(&operation_id, &state.identity)?
                    .ok_or_else(fail)?;
                let saved = committed
                    .remediation
                    .as_ref()
                    .and_then(|p| p.rounds.get(index))
                    .and_then(|r| r.completion.as_ref())
                    .and_then(|i| i.joined_readbacks.as_ref())
                    .ok_or_else(fail)?;
                let expected = campaign_digest(&json!({"completion":completion_locator,
                    "joined":joined}))
                .map_err(|_| fail())?;
                if receipt.kind != "commit_remediation_readbacks"
                    || receipt.request_digest != expected
                    || serde_json::to_value(saved).ok() != serde_json::to_value(joined).ok()
                {
                    return Err(fail());
                }
            }
            let mut evaluated_revisions = std::collections::BTreeSet::new();
            for evaluation in &round.evaluations {
                if !identities.insert(evaluation.operation_id.clone())
                    || !evaluated_revisions
                        .insert((&evaluation.finding_id, &evaluation.finding_revision))
                    || round.outcome.as_ref().is_none_or(|outcome| {
                        outcome["findings"].as_array().is_none_or(|findings| {
                            !findings.iter().any(|finding| {
                                finding["claimId"] == evaluation.finding_id
                                    && finding["revision"] == evaluation.finding_revision
                            })
                        })
                    })
                    || evaluation.consumer_tree != request.candidate.tree
                {
                    return Err(fail());
                }
                let reliance = intent
                    .stages
                    .iter()
                    .find(|s| s.stage_id == format!("{}:reliance", evaluation.operation_id))
                    .ok_or_else(fail)?;
                let projection = intent
                    .stages
                    .iter()
                    .find(|s| s.stage_id == format!("{}:projection", evaluation.operation_id))
                    .ok_or_else(fail)?;
                if reliance.kind != StageKind::EvaluationReliance
                    || projection.kind != StageKind::EvaluationProjection
                    || reliance.registration_operation_id != evaluation.operation_id
                    || projection.registration_operation_id != evaluation.operation_id
                {
                    return Err(fail());
                }
                match evaluation.publication_state.as_str() {
                    "registered"
                        if evaluation.reliance.is_null() && evaluation.projection.is_null() => {}
                    "committed" => {
                        if checked_ref(reliance).map_err(|_| fail())? != evaluation.reliance
                            || checked_ref(projection).map_err(|_| fail())? != evaluation.projection
                        {
                            return Err(fail());
                        }
                        let publication = format!("{}:publish", evaluation.operation_id);
                        if !identities.insert(publication.clone()) {
                            return Err(fail());
                        }
                        let (receipt, committed) = store
                            .projection_result(&publication, &state.identity)?
                            .ok_or_else(fail)?;
                        let saved_round = committed
                            .remediation
                            .as_ref()
                            .and_then(|p| p.rounds.get(index))
                            .ok_or_else(fail)?;
                        let saved = saved_round
                            .evaluations
                            .iter()
                            .find(|r| r.operation_id == evaluation.operation_id)
                            .ok_or_else(fail)?;
                        let view =
                            remediation_evaluation_view(saved_round, saved).map_err(|_| fail())?;
                        if receipt.kind != "publish_remediation_evaluation"
                            || receipt.request_digest
                                != campaign_digest(&view).map_err(|_| fail())?
                            || serde_json::to_value(saved).ok()
                                != serde_json::to_value(evaluation).ok()
                        {
                            return Err(fail());
                        }
                    }
                    _ => return Err(fail()),
                }
            }
        }
        if let Some(outcome) = &round.outcome {
            let intent = round.completion.as_ref().ok_or_else(fail)?;
            let joined = intent.joined_readbacks.as_ref().ok_or_else(fail)?;
            let outcome_stage =
                stage_by_kind(intent, StageKind::InitialOutcome).ok_or_else(fail)?;
            let manifest: RemediationOutcomeManifest =
                decode(&outcome_stage.command).map_err(|_| fail())?;
            let established = joined.builder["status"] == "established"
                && joined.campaign["status"] == "established";
            let published = intent
                .stages
                .iter()
                .filter(|s| s.kind == StageKind::Finding)
                .map(checked_ref)
                .collect::<Result<Vec<_>>>()
                .map_err(|_| fail())?;
            let expected_count = if established {
                manifest.finding_commands.len()
            } else {
                0
            };
            let publication = format!("{}:publish", manifest.operation_id);
            let (receipt, committed) = store
                .projection_result(&publication, &state.identity)?
                .ok_or_else(fail)?;
            let saved = committed
                .remediation
                .as_ref()
                .and_then(|p| p.rounds.get(index))
                .and_then(|r| r.outcome.as_ref())
                .ok_or_else(fail)?;
            if !identities.insert(publication.clone())
                || manifest.established != established
                || published.len() != expected_count
                || outcome["kind"]
                    != (if !established {
                        "evidence_unestablished"
                    } else if outcome["nativePhase"] == "reported" || published.is_empty() {
                        "reported"
                    } else {
                        "awaiting_builder"
                    })
                || outcome["generation"] != request.generation
                || outcome["candidate"]
                    != serde_json::to_value(&request.candidate).map_err(|_| fail())?
                || outcome["episode"] != joined.episode
                || outcome["nativeResultCanonicalJson"] != manifest.native_result_canonical_json
                || outcome["nativeFindingCount"] != manifest.finding_commands.len()
                || outcome["findings"] != serde_json::to_value(&published).map_err(|_| fail())?
                || outcome["publicationOperationId"] != publication
                || (!established && !outcome["projection"].is_null())
                || (established && outcome["projection"].is_null())
                || intent.consumption_intents.len() != 2
                || intent.consumption_intents[0].consumed_at_operation_id
                    != established.then_some(publication.clone())
                || intent.consumption_intents[1]
                    .consumed_at_operation_id
                    .is_some()
                || receipt.kind != "publish_remediation_outcome"
                || receipt.request_digest != campaign_digest(outcome).map_err(|_| fail())?
                || saved != outcome
            {
                return Err(fail());
            }
            for (stage, command) in intent
                .stages
                .iter()
                .filter(|s| s.kind == StageKind::Finding)
                .zip(&manifest.finding_commands)
            {
                if stage.command != serde_json::to_value(command).map_err(|_| fail())? {
                    return Err(fail());
                }
            }
            if established && !published.is_empty() {
                let stage = stage_by_kind(intent, StageKind::FindingProjection).ok_or_else(fail)?;
                if checked_ref(stage).map_err(|_| fail())? != outcome["projection"] {
                    return Err(fail());
                }
            }
        }
        if round.outcome.is_some() && round.dispatch.is_none() {
            return Err(fail());
        }
        prior = &round.request.candidate;
    }
    Ok(())
}
