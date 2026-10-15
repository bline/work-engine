// J2 composition shares the campaign gate; no serialized value is a capability.
use crate::completion::{EvaluationRecord, InitialOutcome};
use claim_evidence::codec::JsString as ClaimString;
use claim_evidence::{
    ClaimId, ClaimProjection, FindingCommand, FindingMutation, FindingReceipt, FindingRevisionId,
    OperationId, OperationLocator, ProjectionConsumer, ProjectionRequest, ProjectionSelection,
    Publication, Reconciliation, RelianceCommand, ReliancePublication, RelianceReconciliation,
};
use serde::Deserialize;

/// This trusted installation cannot be supplied as a deserialized request.
/// ```compile_fail
/// let _: slice_campaign::CompletionSelection = serde_json::from_str("{}").unwrap();
/// ```
/// ```compile_fail
/// use slice_campaign::CheckedInitialOutcome;
/// ```
#[derive(Clone, Debug)]
pub struct CompletionSelection {
    record: CompletionSelectionRecord,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompletionSelectionRecord {
    root: J2Root,
    finding_grant_id: String,
    finding_grant_sha256: String,
    reliance_grant_id: String,
    reliance_grant_sha256: String,
    consumer: String,
    consumer_tree: String,
    decision_scope: String,
    authority_sources: Vec<Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct J2Root {
    path: PathBuf,
    root_id: String,
    profile: String,
}
impl From<&RootIdentity> for J2Root {
    fn from(r: &RootIdentity) -> Self {
        Self {
            path: r.absolute_path.clone(),
            root_id: r.root_id.clone(),
            profile: r.profile.clone(),
        }
    }
}
impl J2Root {
    fn owner(&self) -> RootIdentity {
        RootIdentity {
            absolute_path: self.path.clone(),
            root_id: self.root_id.clone(),
            profile: self.profile.clone(),
        }
    }
}
impl CompletionSelection {
    /// Trusted startup composition only. Requests cannot deserialize an installation.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        root: RootIdentity,
        finding_grant_id: String,
        finding_grant_sha256: String,
        reliance_grant_id: String,
        reliance_grant_sha256: String,
        consumer: String,
        consumer_tree: String,
        decision_scope: String,
        authority_sources: Vec<Value>,
    ) -> Result<Self> {
        let record = CompletionSelectionRecord {
            root: (&root).into(),
            finding_grant_id,
            finding_grant_sha256,
            reliance_grant_id,
            reliance_grant_sha256,
            consumer,
            consumer_tree,
            decision_scope,
            authority_sources,
        };
        record.validate()?;
        Ok(Self { record })
    }
}
impl CompletionSelectionRecord {
    fn validate(&self) -> Result<()> {
        for s in [
            &self.finding_grant_id,
            &self.reliance_grant_id,
            &self.consumer,
            &self.consumer_tree,
            &self.decision_scope,
            &self.root.root_id,
            &self.root.profile,
        ] {
            require_text(s, "completion selection")?;
        }
        crate::require_sha(&self.finding_grant_sha256, "finding grant")?;
        crate::require_sha(&self.reliance_grant_sha256, "reliance grant")?;
        if !self.root.path.is_absolute() || self.authority_sources.is_empty() {
            return Err(j2_error("completion selection authority absent"));
        }
        for source in &self.authority_sources {
            for key in ["owner", "reference", "revision", "freshness"] {
                text_field(source, key)?;
            }
            crate::require_sha(&text_field(source, "integrity_sha256")?, "authority source")?;
            if source["status"] != "verified" {
                return Err(j2_error("authority source not verified"));
            }
        }
        Ok(())
    }
}
impl Campaign {
    pub fn install_completion_selection(&self, selection: CompletionSelection) -> Result<()> {
        selection.record.validate()?;
        let mut gate = self.owner()?;
        let join = gate
            .initial_join
            .as_ref()
            .ok_or_else(|| j2_error("initial join not installed"))?;
        if selection.record.root.owner() != join.selection.claims_root {
            return Err(j2_error("completion claims root differs"));
        }
        if gate
            .completion_selection
            .as_ref()
            .is_some_and(|s| s.record != selection.record)
        {
            return Err(j2_error(
                "completion selection already installed differently",
            ));
        }
        gate.completion_selection = Some(selection);
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InitialOutcomeLocator {
    pub joined: InitialCompletionLocator,
    pub outcome_operation_id: String,
    pub original_registration: RecoveryLocator,
    pub registration_request_sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuilderEvaluationLocator {
    pub joined: InitialCompletionLocator,
    pub evaluation_operation_id: String,
    pub original_registration: RecoveryLocator,
    pub registration_request_sha256: String,
}
#[derive(Clone, Debug)]
pub struct BuilderEvaluationDecision {
    pub claim_id: ClaimId,
    pub finding_revision: FindingRevisionId,
    pub valid: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InitialOutcomeKind {
    EvidenceUnestablished,
    Reported,
    AwaitingBuilder,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InitialOutcomeView {
    pub kind: InitialOutcomeKind,
    pub pair: Value,
    pub episode: Value,
    pub native_result_canonical_json: String,
    pub native_phase: String,
    pub native_verdict: String,
    pub limitations_canonical_json: String,
    pub unresolved_questions_canonical_json: String,
    pub findings: Vec<Value>,
    pub projection: Option<Value>,
    pub consumptions: Vec<ConsumptionIntent>,
    pub initial_publication_operation_id: String,
}
#[allow(clippy::large_enum_variant)] // Public progress carries one bounded checked publication.
#[derive(Clone, Debug)]
pub enum InitialOutcomeState {
    Pending {
        stage: StageKind,
        reason: String,
    },
    Committed {
        view: InitialOutcomeView,
        receipt: OperationReceipt,
    },
    Conflicting {
        stage: StageKind,
        reason: String,
    },
    Unresolved {
        stage: StageKind,
        owner_locator: Value,
        reason: String,
    },
}
#[derive(Clone, Debug)]
pub struct InitialOutcomeProgress {
    pub locator: InitialOutcomeLocator,
    pub last_checked_revision: CampaignRevision,
    pub stages: Vec<StageIntent>,
    pub newly_observed_receipts: Vec<Value>,
    pub state: InitialOutcomeState,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuilderEvaluationView {
    pub operation_id: String,
    pub claim_id: String,
    pub finding_revision: String,
    pub valid: bool,
    pub consumer_tree: String,
    pub decision_scope: String,
    pub reliance: Value,
    pub projection: Value,
    pub outcome: InitialOutcomeView,
}
#[derive(Clone, Debug)]
pub struct BuilderEvaluationPrepared {
    pub snapshot: Snapshot,
    pub locator: BuilderEvaluationLocator,
}
#[allow(clippy::large_enum_variant)] // Same public progress shape as initial completion.
#[derive(Clone, Debug)]
pub enum BuilderEvaluationState {
    Pending {
        stage: StageKind,
        reason: String,
    },
    Committed {
        view: BuilderEvaluationView,
        receipt: OperationReceipt,
    },
    Conflicting {
        stage: StageKind,
        reason: String,
    },
    Unresolved {
        stage: StageKind,
        owner_locator: Value,
        reason: String,
    },
}
#[derive(Clone, Debug)]
pub struct BuilderEvaluationProgress {
    pub locator: BuilderEvaluationLocator,
    pub last_checked_revision: CampaignRevision,
    pub stages: Vec<StageIntent>,
    pub newly_observed_receipts: Vec<Value>,
    pub state: BuilderEvaluationState,
}
// The aggregate has no serde implementation and no public construction route.
struct CheckedInitialOutcome {
    state: Snapshot,
    intent: CompletionIntent,
    manifest: OutcomeManifest,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OutcomeManifest {
    joined: InitialCompletionLocator,
    operation_id: String,
    selection: Option<CompletionSelectionRecord>,
    native_result_canonical_json: String,
    episode_canonical_json: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct J2Admission {
    campaign_root_id: String,
    campaign_revision: String,
    campaign_operation_id: String,
    obligation_id: String,
    candidate_digest: String,
    selection_digest: String,
    profile_digest: String,
    prepared_request_sha256: String,
    episode_revision: String,
    result_sha256: String,
    claims_request_sha256: String,
}
impl From<&AdmissionBinding> for J2Admission {
    fn from(b: &AdmissionBinding) -> Self {
        Self {
            campaign_root_id: b.campaign_root_id.clone(),
            campaign_revision: b.campaign_revision.clone(),
            campaign_operation_id: b.campaign_operation_id.clone(),
            obligation_id: b.obligation_id.clone(),
            candidate_digest: b.candidate_digest.clone(),
            selection_digest: b.selection_digest.clone(),
            profile_digest: b.profile_digest.clone(),
            prepared_request_sha256: b.prepared_request_sha256.clone(),
            episode_revision: b.episode_revision.clone(),
            result_sha256: b.result_sha256.clone(),
            claims_request_sha256: b.claims_request_sha256.clone(),
        }
    }
}
impl J2Admission {
    fn binding(self) -> AdmissionBinding {
        AdmissionBinding {
            campaign_root_id: self.campaign_root_id,
            campaign_revision: self.campaign_revision,
            campaign_operation_id: self.campaign_operation_id,
            obligation_id: self.obligation_id,
            candidate_digest: self.candidate_digest,
            selection_digest: self.selection_digest,
            profile_digest: self.profile_digest,
            prepared_request_sha256: self.prepared_request_sha256,
            episode_revision: self.episode_revision,
            result_sha256: self.result_sha256,
            claims_request_sha256: self.claims_request_sha256,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct J2Locator {
    root: J2Root,
    operation_id: String,
    action: String,
    payload_sha256: String,
    grant_sha256: String,
    admission_sha256: String,
}
impl From<&OperationLocator> for J2Locator {
    fn from(l: &OperationLocator) -> Self {
        Self {
            root: (&l.root).into(),
            operation_id: l.operation_id.as_str().into(),
            action: l.action.into(),
            payload_sha256: l.payload_sha256.clone(),
            grant_sha256: l.grant_sha256.clone(),
            admission_sha256: l.admission_sha256.clone(),
        }
    }
}
impl J2Locator {
    fn owner(self) -> Result<OperationLocator> {
        Ok(OperationLocator {
            root: self.root.owner(),
            operation_id: ce(OperationId::new(self.operation_id))?,
            action: match self.action.as_str() {
                "create_claim" => "create_claim",
                "publish_revision" => "publish_revision",
                "record_reliance" => "record_reliance",
                _ => return Err(j2_error("J2 locator action differs")),
            },
            payload_sha256: self.payload_sha256,
            grant_sha256: self.grant_sha256,
            admission_sha256: self.admission_sha256,
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FindingEnvelope {
    operation_id: String,
    grant_id: String,
    subject_json: String,
    statement_json: String,
    revision_json: String,
    native_id_json: String,
    native_id_digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProjectionEnvelope {
    request_id: String,
    consumer: String,
    consumer_tree: String,
    decision_scope: String,
    revisions: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EvaluationEnvelope {
    operation_id: String,
    grant_id: String,
    claim_id: String,
    finding_revision: String,
    valid: bool,
    consumer: String,
    consumer_tree: String,
    decision_scope: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "payload",
    rename_all = "snake_case",
    deny_unknown_fields
)]
#[allow(clippy::large_enum_variant)] // Closed encoding adapter, short-lived during validation.
enum J2Command {
    Initial(OutcomeManifest),
    Finding(FindingEnvelope),
    Projection(ProjectionEnvelope),
    Consumption(ConsumptionIntent),
    Evaluation(EvaluationEnvelope),
}
fn ce<T>(r: ClaimResult<T>) -> Result<T> {
    r.map_err(|e| CampaignError::Integrity(e.to_string()))
}
fn j2_error(s: &str) -> CampaignError {
    CampaignError::Conflict(s.into())
}
fn encode<T: Serialize>(v: &T) -> Result<Value> {
    serde_json::to_value(v).map_err(|e| CampaignError::Contract(e.to_string()))
}
fn decode<T: serde::de::DeserializeOwned>(v: &Value) -> Result<T> {
    serde_json::from_value(v.clone()).map_err(|e| CampaignError::Integrity(e.to_string()))
}
fn cj(v: &ClaimValue) -> Result<String> {
    ce(claim_json(v))
}
fn cp(s: &str) -> Result<ClaimValue> {
    ce(parse_claim_json(s))
}
fn cf<'a>(v: &'a ClaimValue, k: &str) -> Result<&'a ClaimValue> {
    v.get(k).ok_or_else(|| j2_error("native field absent"))
}
fn manifest(intent: &CompletionIntent) -> Result<OutcomeManifest> {
    match decode::<J2Command>(
        &stage_by_kind(intent, StageKind::InitialOutcome)
            .ok_or_else(|| j2_error("initial outcome registration absent"))?
            .command,
    )? {
        J2Command::Initial(m) => Ok(m),
        _ => Err(j2_error("initial manifest differs")),
    }
}
fn established(intent: &CompletionIntent) -> Result<bool> {
    let joined = intent
        .joined_readbacks
        .as_ref()
        .ok_or_else(|| j2_error("owner reads not joined"))?;
    for v in [&joined.builder, &joined.campaign] {
        if !["established", "false", "unestablished"].contains(&text_field(v, "status")?.as_str()) {
            return Err(j2_error("claim status invalid"));
        }
    }
    Ok(joined.builder["status"] == "established" && joined.campaign["status"] == "established")
}
fn admission_for(
    state: &Snapshot,
    intent: &CompletionIntent,
    operation: &str,
) -> Result<AdmissionBinding> {
    let request = state
        .obligations
        .iter()
        .find(|o| o.obligation_id == intent.obligation_id)
        .and_then(|o| o.request.as_ref())
        .ok_or_else(|| j2_error("request absent"))?;
    Ok(AdmissionBinding {
        campaign_root_id: request.root_id.clone(),
        campaign_revision: state.revision.as_str().into(),
        campaign_operation_id: operation.into(),
        obligation_id: intent.obligation_id.clone(),
        candidate_digest: request.candidate.receipt_sha256.clone(),
        selection_digest: request.selection.campaign_digest.clone(),
        profile_digest: request.profile_digest.clone(),
        prepared_request_sha256: request.request_digest.clone(),
        episode_revision: text_field(
            &intent
                .joined_readbacks
                .as_ref()
                .ok_or_else(|| j2_error("join absent"))?
                .episode,
            "resolvedRevision",
        )?,
        result_sha256: intent.native_result_claim_sha256.clone(),
        claims_request_sha256: String::new(),
    })
}
fn binding_for(stage: &StageIntent) -> Result<AdmissionBinding> {
    Ok(decode::<J2Admission>(&stage.original_admission)?.binding())
}
fn finding_command(e: &FindingEnvelope, admission: AdmissionBinding) -> Result<FindingCommand> {
    Ok(FindingCommand {
        operation_id: ce(OperationId::new(e.operation_id.clone()))?,
        grant_id: e.grant_id.clone(),
        admission,
        mutation: FindingMutation::Create {
            subject: cp(&e.subject_json)?,
            statement_identity: ce(cp(&e.statement_json)?.as_text()).cloned()?,
            initial_revision: cp(&e.revision_json)?,
        },
    })
}
fn projection_command(
    e: &ProjectionEnvelope,
    admission: AdmissionBinding,
) -> Result<ProjectionRequest> {
    Ok(ProjectionRequest {
        request_id: e.request_id.clone(),
        admission,
        consumer: ProjectionConsumer {
            identity: e.consumer.clone(),
            revision: e.consumer_tree.clone(),
            decision_scope: e.decision_scope.clone(),
        },
        selections: e
            .revisions
            .iter()
            .map(|r| {
                Ok(ProjectionSelection {
                    revision_id: ce(FindingRevisionId::new(r.clone()))?,
                    selection_reason:
                        "Exact initial review finding; applicability remains a consumer decision."
                            .into(),
                })
            })
            .collect::<Result<_>>()?,
        limitations: vec!["Publication is not review or campaign acceptance.".into()],
    })
}
fn reliance_command(
    e: &EvaluationEnvelope,
    admission: AdmissionBinding,
) -> Result<RelianceCommand> {
    Ok(RelianceCommand {
        operation_id: ce(OperationId::new(format!("{}:reliance", e.operation_id)))?,
        grant_id: e.grant_id.clone(),
        admission,
        claim_id: ce(ClaimId::new(e.claim_id.clone()))?,
        revision_id: ce(FindingRevisionId::new(e.finding_revision.clone()))?,
        consumer: e.consumer.clone(),
        consumer_revision: e.consumer_tree.clone(),
        decision_scope: e.decision_scope.clone(),
    })
}
#[allow(clippy::too_many_arguments)] // Complete registered owner envelope.
fn make_stage(
    intent: &CompletionIntent,
    state: &Snapshot,
    id: String,
    kind: StageKind,
    operation: &str,
    command: J2Command,
    binding: Option<AdmissionBinding>,
    locator: Value,
) -> Result<StageIntent> {
    let command = encode(&command)?;
    let (request_sha256, original_admission_sha256, original_admission) = if let Some(b) = binding {
        (
            b.claims_request_sha256.clone(),
            b.digest(),
            encode(&J2Admission::from(&b))?,
        )
    } else {
        let d = campaign_digest(&command)?;
        (d.clone(), d, Value::Null)
    };
    Ok(StageIntent {
        stage_id: id,
        kind,
        registration_operation_id: operation.into(),
        registration_base_revision: state.revision.clone(),
        command,
        request_sha256,
        original_admission,
        original_admission_sha256,
        owner_selection_digest: intent.owner_selection_digest.clone(),
        owner_locator: locator,
        evidence_ref: intent.execution_evidence_ref.clone(),
        progress: StageProgress::Registered,
    })
}
fn finding_envelopes(m: &OutcomeManifest) -> Result<Vec<FindingEnvelope>> {
    let result = cp(&m.native_result_canonical_json)?;
    let episode = cp(&m.episode_canonical_json)?;
    let findings = ce(cf(&result, "findings")?.as_array())?;
    if findings.len() > 100 {
        return Err(CampaignError::Contract(
            "initial finding count exceeds 100".into(),
        ));
    }
    if findings.is_empty() {
        return Ok(vec![]);
    }
    let s = m
        .selection
        .as_ref()
        .ok_or_else(|| j2_error("completion selection absent"))?;
    let key = format!(
        "review-episode@{}",
        ce(claim_digest(cf(&episode, "identity")?))?
    );
    let source = ClaimValue::object([
        ("owner", ClaimValue::text("review-episode")),
        ("reference", ClaimValue::text(&key)),
        ("revision", cf(&episode, "revision")?.clone()),
        (
            "integrity_sha256",
            ClaimValue::text(&ce(claim_digest(&episode))?),
        ),
        ("freshness", ClaimValue::text("exact immutable revision")),
        ("status", ClaimValue::text("verified")),
    ]);
    findings
        .iter()
        .map(|f| {
            let native_id = cf(f, "id")?;
            let id_digest = ce(claim_digest(native_id))?;
            let mut stable: Vec<u16> = format!("{key}:").encode_utf16().collect();
            stable.extend(&ce(native_id.as_text())?.0);
            let subject = ClaimValue::object([
                ("namespace", ClaimValue::text("implementation-review")),
                ("subject_kind", ClaimValue::text("revision-bound-finding")),
                ("stable_subject_id", ClaimValue::String(ClaimString(stable))),
                ("evidence_baseline", source.clone()),
                (
                    "content_set",
                    ClaimValue::Array(vec![ClaimValue::text(&key)]),
                ),
            ]);
            let resolved = cf(f, "status")? == &ClaimValue::text("verified_resolved");
            let revision = ClaimValue::object([
                ("proposition", cf(f, "consequence")?.clone()),
                (
                    "support_qualification",
                    ClaimValue::text("attributed_review_finding"),
                ),
                ("assumptions", ClaimValue::Array(vec![])),
                ("limitations", cf(&result, "limitations")?.clone()),
                (
                    "confidence",
                    ClaimValue::object([("label", cf(f, "confidence")?.clone())]),
                ),
                (
                    "evidence_references",
                    ClaimValue::Array(vec![source.clone()]),
                ),
                ("sensitivity_references", ClaimValue::Array(vec![])),
                ("evidence_mode", ClaimValue::text("review_episode_result")),
                ("judgment_kind", ClaimValue::text("review_finding")),
                ("decision_scope", ClaimValue::text(&s.decision_scope)),
                (
                    "profile_payload",
                    ClaimValue::object([
                        ("finding_id", native_id.clone()),
                        ("severity", cf(f, "severity")?.clone()),
                        ("episode", ClaimValue::text(&key)),
                        ("outcome", cf(f, "status")?.clone()),
                    ]),
                ),
                (
                    "reopening_conditions",
                    ClaimValue::Array(if resolved {
                        vec![ClaimValue::text(
                            "new subject revision or contradictory evidence",
                        )]
                    } else {
                        vec![]
                    }),
                ),
                ("tombstone", ClaimValue::Bool(false)),
            ]);
            ce(claim_evidence::stable_claim_id(&subject))?;
            Ok(FindingEnvelope {
                operation_id: format!("{}:finding:{id_digest}", m.operation_id),
                grant_id: s.finding_grant_id.clone(),
                subject_json: cj(&subject)?,
                statement_json: cj(cf(f, "title")?)?,
                revision_json: cj(&revision)?,
                native_id_json: cj(native_id)?,
                native_id_digest: id_digest,
            })
        })
        .collect()
}
fn initial_read_refusal(state: InitialProgressState) -> CampaignError {
    match state {
        InitialProgressState::Pending { stage, reason } => {
            CampaignError::Unsupported(format!("{stage:?}: {reason}"))
        }
        InitialProgressState::Conflicting { stage, reason } => {
            CampaignError::Conflict(format!("{stage:?}: {reason}"))
        }
        InitialProgressState::Unresolved { stage, reason, .. } => {
            CampaignError::Io(format!("{stage:?}: {reason}"))
        }
        InitialProgressState::OwnerReadsJoined => {
            CampaignError::Integrity("unexpected owner progress".into())
        }
    }
}
fn check_initial(
    campaign: &mut Campaign,
    joined: &InitialCompletionLocator,
    operation: &str,
    owners: &mut InitialCompletionOwners<'_>,
) -> Result<CheckedInitialOutcome> {
    checked_owner_wrapper(campaign, owners)?;
    let (_, before) = validate_completion_locator(campaign, joined)?;
    if before.joined_readbacks.is_none() {
        return Err(j2_error("J1 owner join is pending"));
    }
    checked_current_execution(campaign, joined)?;
    for kind in [StageKind::AdmissionBuilder, StageKind::AdmissionCampaign] {
        if let Some(progress) = read_stage(campaign, joined, kind, owners, &[])? {
            return Err(initial_read_refusal(progress.state));
        }
    }
    if let Some(progress) = read_episode_stage(campaign, joined, owners, &[])? {
        return Err(initial_read_refusal(progress.state));
    }
    let (state, intent) = validate_completion_locator(campaign, joined)?;
    let stage = stage_by_kind(&intent, StageKind::EpisodeResultRead)
        .ok_or_else(|| j2_error("episode stage absent"))?;
    let request = episode_request_from_stage(&state, &intent, stage)?;
    let readback = owners
        .episode
        .as_deref_mut()
        .ok_or_else(|| j2_error("episode owner absent"))?
        .read_checked(&request)
        .map_err(|e| j2_error(&e.to_string()))?;
    if readback.observed_revision() != readback.resolved_revision() {
        return Err(j2_error(
            "later episode provenance requires remediation join",
        ));
    }
    let episode = readback
        .state()
        .ok_or_else(|| j2_error("historical episode absent"))?;
    let result = episode
        .current_result()
        .map_err(|e| j2_error(&e.to_string()))?
        .ok_or_else(|| j2_error("native result absent"))?;
    let selection = campaign
        .owner()?
        .completion_selection
        .as_ref()
        .map(|s| s.record.clone());
    if let Some(s) = &selection
        && s.consumer_tree
            != state
                .candidate
                .as_ref()
                .ok_or_else(|| j2_error("candidate absent"))?
                .tree
    {
        return Err(j2_error("builder tree differs from initial candidate"));
    }
    let manifest = OutcomeManifest {
        joined: joined.clone(),
        operation_id: operation.into(),
        selection,
        native_result_canonical_json: review_episode_core::codec::canonical_json(&result.value),
        episode_canonical_json: review_episode_core::codec::canonical_json(episode.value()),
    };
    if let Some(saved) = stage_by_kind(&intent, StageKind::InitialOutcome) {
        let J2Command::Initial(original) = decode(&saved.command)? else {
            return Err(j2_error("manifest kind differs"));
        };
        if encode(&original)? != encode(&manifest)? {
            return Err(j2_error("registered J2 owner selection or result changed"));
        }
    }
    // Recompute commitments using the checked readbacks, including full establishment refs.
    let joined_reads = intent.joined_readbacks.as_ref().unwrap();
    for (index, record) in [&joined_reads.builder, &joined_reads.campaign]
        .iter()
        .enumerate()
    {
        let original = intent
            .consumption_intents
            .get(index)
            .ok_or_else(|| j2_error("consumption absent"))?;
        let mut expected = consumption_commitment(record, &original.registration_operation_id)?;
        expected.consumed_at_operation_id = original.consumed_at_operation_id.clone();
        if encode(&expected)? != encode(original)? {
            return Err(j2_error("consumption pair changed"));
        }
    }
    Ok(CheckedInitialOutcome {
        state,
        intent,
        manifest,
    })
}
fn initial_registration_digest(
    state: &Snapshot,
    stages: &[StageIntent],
    operation: &str,
) -> Result<String> {
    campaign_digest(
        &json!({"identity":state.identity,"expectedRevision":state.revision,"operationId":operation,"stages":stages}),
    )
}
fn initial_locator(
    gate: &CampaignInner,
    m: &OutcomeManifest,
    receipt: &OperationReceipt,
) -> InitialOutcomeLocator {
    InitialOutcomeLocator {
        joined: m.joined.clone(),
        outcome_operation_id: m.operation_id.clone(),
        registration_request_sha256: receipt.request_digest.clone(),
        original_registration: gate.locator(
            &m.joined.identity,
            &m.operation_id,
            "register_initial_outcome",
            receipt.request_digest.clone(),
            receipt.prior_revision.clone(),
        ),
    }
}
fn outcome_stages(
    checked: &CheckedInitialOutcome,
    owners: &InitialCompletionOwners<'_>,
) -> Result<Vec<StageIntent>> {
    let m = &checked.manifest;
    let state = &checked.state;
    let intent = &checked.intent;
    let mut stages = vec![make_stage(
        intent,
        state,
        format!("{}:initial-outcome", m.operation_id),
        StageKind::InitialOutcome,
        &m.operation_id,
        J2Command::Initial(m.clone()),
        None,
        json!({"operationId":m.operation_id}),
    )?];
    for (index, c) in intent.consumption_intents.iter().enumerate() {
        stages.push(make_stage(
            intent,
            state,
            format!("{}:consumption:{index}", m.operation_id),
            if index == 0 {
                StageKind::ConsumptionBuilder
            } else {
                StageKind::ConsumptionCampaign
            },
            &m.operation_id,
            J2Command::Consumption(c.clone()),
            None,
            consumption_reference(c),
        )?);
    }
    // Count and codec preflight applies even when evidence is not usable.
    let result = cp(&m.native_result_canonical_json)?;
    if ce(cf(&result, "findings")?.as_array())?.len() > 100 {
        return Err(CampaignError::Contract(
            "initial finding count exceeds 100".into(),
        ));
    }
    if established(intent)? {
        for e in finding_envelopes(m)? {
            let mut command = finding_command(&e, admission_for(state, intent, &m.operation_id)?)?;
            command.admission.claims_request_sha256 =
                ce(claim_evidence::claims_request_sha256(&command))?;
            let locator = ce(owners.claims.claims.locator(&command))?;
            if locator.grant_sha256 != m.selection.as_ref().unwrap().finding_grant_sha256
                || locator.root != m.selection.as_ref().unwrap().root.owner()
            {
                return Err(j2_error("finding grant or root differs"));
            }
            stages.push(make_stage(
                intent,
                state,
                e.operation_id.clone(),
                StageKind::Finding,
                &m.operation_id,
                J2Command::Finding(e),
                Some(command.admission),
                encode(&J2Locator::from(&locator))?,
            )?);
        }
    }
    // Bound subsequent receipts, exact projection, outcome and original result copies.
    let stage_bytes = serde_json::to_vec(&stages)
        .map_err(|e| j2_error(&e.to_string()))?
        .len();
    let state_bytes = serde_json::to_vec(state)
        .map_err(|e| j2_error(&e.to_string()))?
        .len();
    if state_bytes
        .saturating_add(stage_bytes.saturating_mul(4))
        .saturating_add(65_536)
        > 1_048_576
    {
        return Err(CampaignError::Contract(
            "J2 complete publication exceeds campaign capacity preflight".into(),
        ));
    }
    Ok(stages)
}
impl Campaign {
    pub fn finalize_initial_completion(
        &mut self,
        identity: &CampaignIdentity,
        expected: &CampaignRevision,
        joined: &InitialCompletionLocator,
        outcome_operation_id: &str,
        owners: &mut InitialCompletionOwners<'_>,
    ) -> Result<InitialOutcomeProgress> {
        require_text(outcome_operation_id, "outcome operation")?;
        if identity != &joined.identity {
            return Err(j2_error("outcome identity differs"));
        }
        if let Some(locator) =
            self.locate_initial_outcome(identity, &joined.obligation_id, outcome_operation_id)?
        {
            if encode(&locator.joined)? != encode(joined)? {
                return Err(j2_error("outcome join differs"));
            }
            return self.recover_initial_outcome(&locator, owners);
        }
        self.owner()?.current(identity, expected)?;
        let checked = check_initial(self, joined, outcome_operation_id, owners)?;
        if checked.intent.outcome.is_some()
            || stage_by_kind(&checked.intent, StageKind::InitialOutcome).is_some()
        {
            return Err(j2_error("another outcome is already registered"));
        }
        let stages = outcome_stages(&checked, owners)?;
        let digest = initial_registration_digest(&checked.state, &stages, outcome_operation_id)?;
        let mut next = checked.state.clone();
        current_intent_mut(
            &mut next,
            &joined.obligation_id,
            &joined.completion_operation_id,
        )?
        .stages
        .extend(stages);
        next.revision = state_revision(&next)?;
        preflight_snapshot(&next)?;
        let mut gate = self.owner()?;
        gate.current(identity, expected)?;
        let receipt = gate.receipt(
            &next,
            Some(expected.clone()),
            outcome_operation_id,
            "register_initial_outcome",
            digest.clone(),
        );
        let locator = initial_locator(&gate, &checked.manifest, &receipt);
        let effect = gate.put(
            next,
            Some(expected.clone()),
            outcome_operation_id,
            "register_initial_outcome",
            digest,
            false,
            None,
        );
        drop(gate);
        match effect {
            Effect::Applied { value, .. } | Effect::Replayed { value, .. } => {
                crate::fault_cut("j2_after_outcome_registration");
                match drive_outcome(self, &locator, owners) {
                    Ok(mut p) => {
                        p.newly_observed_receipts.insert(0, encode(&receipt)?);
                        Ok(p)
                    }
                    Err(e) => Ok(outcome_unresolved(
                        &locator,
                        &value,
                        StageKind::InitialOutcome,
                        e.to_string(),
                        vec![encode(&receipt)?],
                    )),
                }
            }
            Effect::OutcomeUnknown(l) => Ok(outcome_unresolved(
                &locator,
                &checked.state,
                StageKind::InitialOutcome,
                format!("registration uncertain: {l:?}"),
                vec![],
            )),
            Effect::NoEffect(reason) => Err(j2_error(&reason)),
        }
    }
    pub fn locate_initial_outcome(
        &self,
        identity: &CampaignIdentity,
        obligation_id: &str,
        outcome_operation_id: &str,
    ) -> Result<Option<InitialOutcomeLocator>> {
        let gate = self.owner()?;
        gate.check_identity(identity)?;
        let Some(state) = gate.store.read(identity)? else {
            return Ok(None);
        };
        if !state
            .obligations
            .iter()
            .any(|o| o.obligation_id == obligation_id)
        {
            return Err(j2_error("outcome obligation absent"));
        }
        let Some(receipt) = gate.store.receipt(outcome_operation_id)? else {
            return Ok(None);
        };
        let intent = state
            .progress
            .attempts
            .iter()
            .filter_map(|a| a.completion_intent.as_ref())
            .find(|i| i.obligation_id == obligation_id)
            .ok_or_else(|| j2_error("outcome intent absent"))?;
        let m = manifest(intent)?;
        if m.operation_id != outcome_operation_id || receipt.kind != "register_initial_outcome" {
            return Err(j2_error("outcome registration conflicts"));
        }
        Ok(Some(initial_locator(&gate, &m, &receipt)))
    }
    pub fn recover_initial_outcome(
        &mut self,
        locator: &InitialOutcomeLocator,
        owners: &mut InitialCompletionOwners<'_>,
    ) -> Result<InitialOutcomeProgress> {
        validate_outcome_locator(self, locator)?;
        drive_outcome(self, locator, owners)
    }
}
fn validate_outcome_locator(c: &Campaign, l: &InitialOutcomeLocator) -> Result<Snapshot> {
    let found = c
        .locate_initial_outcome(
            &l.joined.identity,
            &l.joined.obligation_id,
            &l.outcome_operation_id,
        )?
        .ok_or_else(|| j2_error("outcome registration absent"))?;
    if encode(&found)? != encode(l)? {
        return Err(j2_error("outcome locator differs"));
    }
    let (state, _) = validate_completion_locator(c, &l.joined)?;
    Ok(state)
}
fn outcome_unresolved(
    l: &InitialOutcomeLocator,
    state: &Snapshot,
    stage: StageKind,
    reason: String,
    newly: Vec<Value>,
) -> InitialOutcomeProgress {
    let stages = state
        .progress
        .attempts
        .iter()
        .find(|a| a.attempt_id == l.joined.attempt_id)
        .and_then(|a| a.completion_intent.as_ref())
        .map(|i| i.stages.clone())
        .unwrap_or_default();
    InitialOutcomeProgress {
        locator: l.clone(),
        last_checked_revision: state.revision.clone(),
        stages,
        newly_observed_receipts: newly,
        state: InitialOutcomeState::Unresolved {
            stage,
            owner_locator: encode(l).unwrap_or(Value::Null),
            reason,
        },
    }
}
fn j2_progress_commit(
    c: &mut Campaign,
    joined: &InitialCompletionLocator,
    id: &str,
    progress: StageProgress,
) -> Result<Snapshot> {
    let mut gate = c.owner()?;
    let state = gate
        .store
        .read(&joined.identity)?
        .ok_or_else(|| j2_error("campaign absent"))?;
    let intent = current_intent(
        &state,
        &joined.obligation_id,
        &joined.completion_operation_id,
    )?;
    let saved = intent
        .stages
        .iter()
        .find(|s| s.stage_id == id)
        .ok_or_else(|| j2_error("J2 stage absent"))?;
    if encode(&saved.progress)? == encode(&progress)? {
        return Ok(state);
    }
    if !matches!(saved.progress, StageProgress::Registered)
        && !matches!(
            saved.kind,
            StageKind::FindingProjection | StageKind::EvaluationProjection
        )
    {
        return Err(j2_error("saved J2 receipt differs"));
    }
    let operation = j2_progress_operation(id, saved.kind, &progress)?;
    let digest = campaign_digest(&json!({"stageId":id,"progress":progress}))?;
    let mut next = state.clone();
    current_intent_mut(
        &mut next,
        &joined.obligation_id,
        &joined.completion_operation_id,
    )?
    .stages
    .iter_mut()
    .find(|s| s.stage_id == id)
    .unwrap()
    .progress = progress;
    next.revision = state_revision(&next)?;
    preflight_snapshot(&next)?;
    match gate.put(
        next,
        Some(state.revision),
        &operation,
        "check_j2_stage",
        digest,
        false,
        None,
    ) {
        Effect::Applied { value, .. } | Effect::Replayed { value, .. } => Ok(value),
        Effect::NoEffect(e) => Err(j2_error(&e)),
        Effect::OutcomeUnknown(l) => Err(CampaignError::Io(format!(
            "stage progress uncertain: {l:?}"
        ))),
    }
}
fn checked_progress(v: Value) -> Result<StageProgress> {
    Ok(StageProgress::Checked {
        content_digests: vec![campaign_digest(&v)?],
        exact_refs: vec![v],
    })
}
fn finding_receipt(r: &FindingReceipt, e: &FindingEnvelope) -> Result<Value> {
    let reference = json!({"owner":"claim-evidence","reference":r.claim_id.as_str(),"revision":r.revision_id.as_str(),"sha256":r.revision_sha256,"freshness":"exact immutable revision"});
    let payload = cp(&e.revision_json)?;
    Ok(
        json!({"claimId":r.claim_id.as_str(),"revision":r.revision_id.as_str(),"revisionSha256":r.revision_sha256,
        "locator":encode(&J2Locator::from(&r.locator))?,"nativeIdCanonicalJson":e.native_id_json,"nativeIdDigest":e.native_id_digest,
        "initialRevisionRef":reference,"revisionRef":reference,
        "outcomeCanonicalJson":cj(cf(cf(&payload,"profile_payload")?,"outcome")?)?}),
    )
}
fn checked_ref(stage: &StageIntent) -> Result<Value> {
    checked_record_from_stage(stage)
}
fn projection_envelope(
    m: &OutcomeManifest,
    revisions: Vec<String>,
    request_id: String,
) -> Result<ProjectionEnvelope> {
    let s = m
        .selection
        .as_ref()
        .ok_or_else(|| j2_error("completion selection absent"))?;
    Ok(ProjectionEnvelope {
        request_id,
        consumer: s.consumer.clone(),
        consumer_tree: s.consumer_tree.clone(),
        decision_scope: s.decision_scope.clone(),
        revisions,
    })
}
fn register_projection(
    c: &mut Campaign,
    joined: &InitialCompletionLocator,
    m: &OutcomeManifest,
    kind: StageKind,
    operation: &str,
    envelope: ProjectionEnvelope,
) -> Result<StageIntent> {
    let mut gate = c.owner()?;
    let state = gate
        .store
        .read(&joined.identity)?
        .ok_or_else(|| j2_error("campaign absent"))?;
    let intent = current_intent(
        &state,
        &joined.obligation_id,
        &joined.completion_operation_id,
    )?;
    let id = envelope.request_id.clone();
    if let Some(stage) = intent.stages.iter().find(|s| s.stage_id == id) {
        return Ok(stage.clone());
    }
    let mut request = projection_command(&envelope, admission_for(&state, intent, operation)?)?;
    request.admission.claims_request_sha256 =
        ce(claim_evidence::projection_request_sha256(&request))?;
    let stage = make_stage(
        intent,
        &state,
        id,
        kind,
        operation,
        J2Command::Projection(envelope),
        Some(request.admission),
        encode(&m.selection.as_ref().unwrap().root)?,
    )?;
    let mut next = state.clone();
    current_intent_mut(
        &mut next,
        &joined.obligation_id,
        &joined.completion_operation_id,
    )?
    .stages
    .push(stage.clone());
    next.revision = state_revision(&next)?;
    preflight_snapshot(&next)?;
    let digest = campaign_digest(
        &json!({"identity":state.identity,"expectedRevision":state.revision,"stage":stage}),
    )?;
    match gate.put(
        next,
        Some(state.revision),
        operation,
        "register_j2_projection",
        digest,
        false,
        None,
    ) {
        Effect::Applied { .. } | Effect::Replayed { .. } => Ok(stage),
        Effect::NoEffect(e) => Err(j2_error(&e)),
        Effect::OutcomeUnknown(l) => Err(CampaignError::Io(format!(
            "projection registration uncertain: {l:?}"
        ))),
    }
}
fn projection_value(p: &ClaimProjection) -> Result<Value> {
    let v = p.to_js_value();
    Ok(
        json!({"kind":"ce_projection","requestId":p.request_id(),"projectionCanonicalJson":cj(&v)?,"projectionSha256":ce(claim_digest(&v))?}),
    )
}
fn empty_projection(intent: &CompletionIntent) -> Result<Value> {
    let joined = intent
        .joined_readbacks
        .as_ref()
        .ok_or_else(|| j2_error("joined readbacks absent"))?;
    let body = json!({"schemaVersion":1,"kind":"no_findings","initialEpisodeRef":joined.episode,"nativeResultClaimSha256":intent.native_result_claim_sha256,"findingSetSha256":ce(claim_digest(&ClaimValue::Array(vec![])))?,"count":0});
    let digest = ce(claim_digest(&claim_from_value(&body)?))?;
    Ok(
        json!({"kind":"no_findings","owner":"slice-campaign","reference":format!("no-findings:{digest}"),"body":body}),
    )
}
fn build_view(
    state: &Snapshot,
    intent: &CompletionIntent,
    m: &OutcomeManifest,
    projection: Option<Value>,
) -> Result<InitialOutcomeView> {
    let result = cp(&m.native_result_canonical_json)?;
    let ep = cp(&m.episode_canonical_json)?;
    let findings = intent
        .stages
        .iter()
        .filter(|s| s.kind == StageKind::Finding)
        .map(checked_ref)
        .collect::<Result<Vec<_>>>()?;
    let usable = established(intent)?;
    let native_phase = claim_text_field(&ep, "phase")?;
    let native_findings = ce(cf(&result, "findings")?.as_array())?;
    let all_done = !findings.is_empty()
        && findings.iter().all(|f| {
            state.progress.evaluations.iter().any(|e| {
                e.publication_state == "committed"
                    && e.finding_id == f["claimId"]
                    && e.finding_revision == f["revision"]
                    && (!e.valid || {
                        let id = f["nativeIdCanonicalJson"].as_str().and_then(|s| cp(s).ok());
                        native_findings.iter().any(|v| {
                            v.get("id") == id.as_ref()
                                && v.get("status") == Some(&ClaimValue::text("verified_resolved"))
                        })
                    })
            })
        });
    let kind = if !usable {
        InitialOutcomeKind::EvidenceUnestablished
    } else if native_phase == "reported" || findings.is_empty() || all_done {
        InitialOutcomeKind::Reported
    } else {
        InitialOutcomeKind::AwaitingBuilder
    };
    let joined = intent.joined_readbacks.as_ref().unwrap();
    Ok(InitialOutcomeView {
        kind,
        pair: json!({"builder":joined.builder,"campaign":joined.campaign,"selectedClaims":intent.selected_claims}),
        episode: joined.episode.clone(),
        native_result_canonical_json: m.native_result_canonical_json.clone(),
        native_phase,
        native_verdict: claim_text_field(&result, "verdict")?,
        limitations_canonical_json: cj(cf(&result, "limitations")?)?,
        unresolved_questions_canonical_json: cj(cf(&ep, "unresolvedQuestions")?)?,
        findings,
        projection,
        consumptions: intent.consumption_intents.clone(),
        initial_publication_operation_id: format!("{}:publish", m.operation_id),
    })
}
fn stored_outcome(view: &InitialOutcomeView) -> Result<InitialOutcome> {
    Ok(match view.kind {
        InitialOutcomeKind::EvidenceUnestablished => InitialOutcome::EvidenceUnestablished {
            pair: view.pair.clone(),
        },
        InitialOutcomeKind::Reported => InitialOutcome::Reported {
            pair: view.pair.clone(),
            consumptions: encode(&view.consumptions)?,
            episode: view.episode.clone(),
            finding_set: encode(&view.findings)?,
            projection: view
                .projection
                .clone()
                .ok_or_else(|| j2_error("usable projection absent"))?,
        },
        InitialOutcomeKind::AwaitingBuilder => InitialOutcome::AwaitingBuilder {
            pair: view.pair.clone(),
            consumptions: encode(&view.consumptions)?,
            episode: view.episode.clone(),
            finding_set: encode(&view.findings)?,
            projection: view
                .projection
                .clone()
                .ok_or_else(|| j2_error("usable projection absent"))?,
        },
    })
}
fn saved_projection(intent: &CompletionIntent) -> Result<Option<Value>> {
    Ok(match &intent.outcome {
        Some(
            InitialOutcome::Reported { projection, .. }
            | InitialOutcome::AwaitingBuilder { projection, .. },
        ) => Some(projection.clone()),
        Some(InitialOutcome::EvidenceUnestablished { .. }) => None,
        _ => return Err(j2_error("published outcome absent")),
    })
}
fn drive_outcome(
    c: &mut Campaign,
    l: &InitialOutcomeLocator,
    owners: &mut InitialCompletionOwners<'_>,
) -> Result<InitialOutcomeProgress> {
    let mut last = validate_outcome_locator(c, l)?;
    let mut newly = vec![];
    let mut active = StageKind::InitialOutcome;
    let run = (|| -> Result<(InitialOutcomeView, OperationReceipt)> {
        let checked = check_initial(c, &l.joined, &l.outcome_operation_id, owners)?;
        let m = checked.manifest;
        last = checked.state;
        if checked.intent.outcome.is_some() {
            verify_committed_outcome_owners(&last, &checked.intent, owners)?;
            let view = build_view(
                &last,
                &checked.intent,
                &m,
                saved_projection(&checked.intent)?,
            )?;
            let receipt = c
                .owner()?
                .store
                .receipt(&view.initial_publication_operation_id)?
                .ok_or_else(|| j2_error("initial publication receipt absent"))?;
            return Ok((view, receipt));
        }
        for stage in checked
            .intent
            .stages
            .iter()
            .filter(|s| s.kind == StageKind::Finding)
        {
            active = StageKind::Finding;
            let J2Command::Finding(e) = decode(&stage.command)? else {
                return Err(j2_error("finding command differs"));
            };
            let locator = decode::<J2Locator>(&stage.owner_locator)?.owner()?;
            let command = finding_command(&e, binding_for(stage)?)?;
            if ce(owners.claims.claims.locator(&command))? != locator {
                return Err(j2_error("finding owner locator differs"));
            }
            let receipt = match owners.claims.claims.reconcile(&locator) {
                Reconciliation::Committed(r) => *r,
                Reconciliation::Absent => match owners.claims.claims.publish(command) {
                    Ok(Publication::Applied(r) | Publication::Replayed(r)) => r,
                    Err(claim_evidence::WriteError::NoEffect(e)) => {
                        return Err(j2_error(&e.to_string()));
                    }
                    Err(claim_evidence::WriteError::OutcomeUnknown(locator)) => {
                        return Err(CampaignError::Io(format!(
                            "finding publication uncertain: {locator:?}"
                        )));
                    }
                },
                Reconciliation::Conflicting => {
                    return Err(j2_error("finding owner record conflicts"));
                }
                Reconciliation::Unresolved(reason) => return Err(CampaignError::Io(reason)),
            };
            let value = finding_receipt(&receipt, &e)?;
            newly.push(value.clone());
            crate::fault_cut("j2_after_finding_commit");
            last = j2_progress_commit(c, &l.joined, &stage.stage_id, checked_progress(value)?)?;
        }
        let intent = current_intent(
            &last,
            &l.joined.obligation_id,
            &l.joined.completion_operation_id,
        )?
        .clone();
        let mut fresh = None;
        let projection = if !established(&intent)? {
            None
        } else {
            let findings = intent
                .stages
                .iter()
                .filter(|s| s.kind == StageKind::Finding)
                .map(checked_ref)
                .collect::<Result<Vec<_>>>()?;
            if findings.is_empty() {
                Some(empty_projection(&intent)?)
            } else {
                active = StageKind::FindingProjection;
                let e = projection_envelope(
                    &m,
                    findings
                        .iter()
                        .map(|f| text_field(f, "revision"))
                        .collect::<Result<_>>()?,
                    format!("{}:projection", m.operation_id),
                )?;
                let stage = register_projection(
                    c,
                    &l.joined,
                    &m,
                    StageKind::FindingProjection,
                    &format!("{}:projection-register", m.operation_id),
                    e.clone(),
                )?;
                let p = ce(owners
                    .claims
                    .claims
                    .project_exact_revisions(&projection_command(&e, binding_for(&stage)?)?))?;
                let value = projection_value(&p)?;
                newly.push(value.clone());
                crate::fault_cut("j2_after_projection_read");
                last = j2_progress_commit(
                    c,
                    &l.joined,
                    &stage.stage_id,
                    checked_progress(value.clone())?,
                )?;
                fresh = Some(p);
                Some(value)
            }
        };
        if let Some(p) = &fresh {
            ce(owners.claims.claims.verify_projection_fresh(p))?;
        }
        active = StageKind::InitialOutcome;
        let mut gate = c.owner()?;
        let current = gate.current(&l.joined.identity, &last.revision)?;
        let mut next = current.clone();
        if established(&intent)? {
            current_intent_mut(
                &mut next,
                &l.joined.obligation_id,
                &l.joined.completion_operation_id,
            )?
            .consumption_intents[0]
                .consumed_at_operation_id = Some(format!("{}:publish", m.operation_id));
        }
        let view = build_view(
            &next,
            current_intent(
                &next,
                &l.joined.obligation_id,
                &l.joined.completion_operation_id,
            )?,
            &m,
            projection,
        )?;
        let outcome = stored_outcome(&view)?;
        current_intent_mut(
            &mut next,
            &l.joined.obligation_id,
            &l.joined.completion_operation_id,
        )?
        .outcome = Some(outcome);
        let usable = view.kind != InitialOutcomeKind::EvidenceUnestablished;
        next.progress
            .attempts
            .iter_mut()
            .find(|a| a.attempt_id == l.joined.attempt_id)
            .unwrap()
            .outcome = Some(if usable {
            AttemptOutcome::Completed
        } else {
            AttemptOutcome::FailedWithResult
        });
        next.obligations
            .iter_mut()
            .find(|o| o.obligation_id == l.joined.obligation_id)
            .unwrap()
            .status = if usable { "completed" } else { "failed" }.into();
        next.revision = state_revision(&next)?;
        preflight_snapshot(&next)?;
        let digest = campaign_digest(&encode(&view)?)?;
        crate::fault_cut("j2_before_outcome_commit");
        match gate.put(
            next,
            Some(current.revision),
            &view.initial_publication_operation_id,
            "publish_initial_outcome",
            digest,
            false,
            None,
        ) {
            Effect::Applied { value, receipt } | Effect::Replayed { value, receipt } => {
                last = value;
                newly.push(encode(&receipt)?);
                crate::fault_cut("j2_after_outcome_commit");
                Ok((view, receipt))
            }
            Effect::NoEffect(e) => Err(j2_error(&e)),
            Effect::OutcomeUnknown(x) => Err(CampaignError::Io(format!(
                "outcome publication uncertain: {x:?}"
            ))),
        }
    })();
    match run {
        Ok((view, receipt)) => Ok(InitialOutcomeProgress {
            locator: l.clone(),
            last_checked_revision: last.revision.clone(),
            stages: current_intent(
                &last,
                &l.joined.obligation_id,
                &l.joined.completion_operation_id,
            )?
            .stages
            .clone(),
            newly_observed_receipts: newly,
            state: InitialOutcomeState::Committed { view, receipt },
        }),
        Err(e) => {
            let mut progress = outcome_unresolved(l, &last, active, e.to_string(), newly);
            progress.state = match e {
                CampaignError::Unsupported(reason) => InitialOutcomeState::Pending {
                    stage: active,
                    reason,
                },
                CampaignError::Conflict(reason) => InitialOutcomeState::Conflicting {
                    stage: active,
                    reason,
                },
                _ => progress.state,
            };
            Ok(progress)
        }
    }
}
pub(crate) fn j2_registration_kind(kind: StageKind) -> &'static str {
    match kind {
        StageKind::FindingProjection => "register_j2_projection",
        StageKind::EvaluationReliance
        | StageKind::EvaluationProjection
        | StageKind::EvaluationCommit => "prepare_builder_evaluation",
        _ => "register_initial_outcome",
    }
}
pub(crate) fn validate_j2_stage(
    state: &Snapshot,
    intent: &CompletionIntent,
    stage: &StageIntent,
) -> Result<()> {
    let command: J2Command = decode(&stage.command)?;
    let invalid = || j2_error("saved J2 stage differs");
    if encode(&command)? != stage.command
        || stage.owner_selection_digest != intent.owner_selection_digest
        || stage.evidence_ref != intent.execution_evidence_ref
    {
        return Err(invalid());
    }
    let expected_id = match &command {
        J2Command::Initial(m) if stage.kind == StageKind::InitialOutcome => {
            if m.joined.attempt_id != intent.attempt_id
                || m.joined.completion_operation_id != intent.campaign_operation_id
                || m.operation_id != stage.registration_operation_id
            {
                return Err(invalid());
            }
            let result = cp(&m.native_result_canonical_json)?;
            if ce(claim_digest(&result))? != intent.native_result_claim_sha256 {
                return Err(invalid());
            }
            let ep = cp(&m.episode_canonical_json)?;
            // Native parser validates the complete closed result and episode codec on reads.
            ReviewResult::parse(
                parse_episode_json(&m.native_result_canonical_json)
                    .map_err(|e| j2_error(&e.to_string()))?,
            )
            .map_err(|e| j2_error(&e.to_string()))?;
            if ce(cf(&result, "findings")?.as_array())?.len() > 100
                || cf(&ep, "revision")?
                    != &ClaimValue::text(&text_field(
                        &intent
                            .joined_readbacks
                            .as_ref()
                            .ok_or_else(invalid)?
                            .episode,
                        "resolvedRevision",
                    )?)
            {
                return Err(invalid());
            }
            if let Some(s) = &m.selection {
                s.validate()?;
                if state
                    .candidate
                    .as_ref()
                    .is_none_or(|c| c.tree != s.consumer_tree)
                {
                    return Err(invalid());
                }
            }
            format!("{}:initial-outcome", m.operation_id)
        }
        J2Command::Consumption(c)
            if matches!(
                stage.kind,
                StageKind::ConsumptionBuilder | StageKind::ConsumptionCampaign
            ) =>
        {
            let index = usize::from(stage.kind == StageKind::ConsumptionCampaign);
            let mut expected = intent
                .consumption_intents
                .get(index)
                .ok_or_else(invalid)?
                .clone();
            expected.consumed_at_operation_id = None;
            if encode(&expected)? != encode(c)? {
                return Err(invalid());
            }
            format!("{}:consumption:{index}", stage.registration_operation_id)
        }
        J2Command::Finding(e) if stage.kind == StageKind::Finding => {
            if !established(intent)?
                || ce(claim_digest(&cp(&e.native_id_json)?))? != e.native_id_digest
            {
                return Err(invalid());
            }
            e.operation_id.clone()
        }
        J2Command::Projection(e)
            if matches!(
                stage.kind,
                StageKind::FindingProjection | StageKind::EvaluationProjection
            ) =>
        {
            e.request_id.clone()
        }
        J2Command::Evaluation(e) if stage.kind == StageKind::EvaluationReliance => {
            format!("{}:reliance", e.operation_id)
        }
        _ => return Err(invalid()),
    };
    if stage.stage_id != expected_id || stage.owner_locator.is_null() {
        return Err(invalid());
    }
    match &command {
        J2Command::Finding(e) => {
            let b = binding_for(stage)?;
            validate_j2_binding(state, intent, stage, &b)?;
            let command = finding_command(e, b)?;
            if ce(claim_evidence::claims_request_sha256(&command))? != stage.request_sha256 {
                return Err(invalid());
            }
            let l = decode::<J2Locator>(&stage.owner_locator)?.owner()?;
            let m = manifest(intent)?;
            let s = m.selection.ok_or_else(invalid)?;
            if l.root != s.root.owner()
                || l.grant_sha256 != s.finding_grant_sha256
                || l.admission_sha256 != stage.original_admission_sha256
                || l.operation_id != command.operation_id
                || l.action != "create_claim"
            {
                return Err(invalid());
            }
        }
        J2Command::Evaluation(e) => {
            let b = binding_for(stage)?;
            validate_j2_binding(state, intent, stage, &b)?;
            let command = reliance_command(e, b)?;
            if ce(claim_evidence::reliance_request_sha256(&command))? != stage.request_sha256 {
                return Err(invalid());
            }
            let s = manifest(intent)?.selection.ok_or_else(invalid)?;
            let l = decode::<J2Locator>(&stage.owner_locator)?.owner()?;
            if e.grant_id != s.reliance_grant_id
                || e.consumer != s.consumer
                || e.consumer_tree != s.consumer_tree
                || e.decision_scope != s.decision_scope
                || l.root != s.root.owner()
                || l.grant_sha256 != s.reliance_grant_sha256
                || l.admission_sha256 != stage.original_admission_sha256
                || l.operation_id != command.operation_id
            {
                return Err(invalid());
            }
        }
        J2Command::Projection(e) => {
            let b = binding_for(stage)?;
            validate_j2_binding(state, intent, stage, &b)?;
            if ce(claim_evidence::projection_request_sha256(
                &projection_command(e, b)?,
            ))? != stage.request_sha256
            {
                return Err(invalid());
            }
            let s = manifest(intent)?.selection.ok_or_else(invalid)?;
            if e.consumer != s.consumer
                || e.consumer_tree != s.consumer_tree
                || e.decision_scope != s.decision_scope
                || stage.owner_locator != encode(&s.root)?
            {
                return Err(invalid());
            }
            let all = intent
                .stages
                .iter()
                .filter(|s| s.kind == StageKind::Finding)
                .map(|s| text_field(&checked_ref(s)?, "revision"))
                .collect::<Result<Vec<_>>>()?;
            if e.revisions != all {
                return Err(invalid());
            }
        }
        _ => {
            if stage.original_admission != Value::Null
                || stage.request_sha256 != campaign_digest(&stage.command)?
                || stage.original_admission_sha256 != stage.request_sha256
            {
                return Err(invalid());
            }
        }
    }
    match &stage.progress {
        StageProgress::Registered => {}
        StageProgress::Checked {
            exact_refs,
            content_digests,
        } => {
            if exact_refs.len() != 1
                || content_digests.len() != 1
                || campaign_digest(&exact_refs[0])? != content_digests[0]
            {
                return Err(invalid());
            }
            let value = &exact_refs[0];
            match command {
                J2Command::Finding(e) => {
                    if value["locator"] != stage.owner_locator
                        || value["nativeIdCanonicalJson"] != e.native_id_json
                        || value["nativeIdDigest"] != e.native_id_digest
                        || value["claimId"]
                            != ce(claim_evidence::stable_claim_id(&cp(&e.subject_json)?))?.as_str()
                    {
                        return Err(invalid());
                    }
                    let receipt = FindingReceipt {
                        claim_id: ce(ClaimId::new(text_field(value, "claimId")?))?,
                        revision_id: ce(FindingRevisionId::new(text_field(value, "revision")?))?,
                        revision_sha256: text_field(value, "revisionSha256")?,
                        locator: decode::<J2Locator>(&stage.owner_locator)?.owner()?,
                    };
                    crate::require_sha(&receipt.revision_sha256, "finding revision")?;
                    if finding_receipt(&receipt, &e)? != *value {
                        return Err(invalid());
                    }
                }
                J2Command::Projection(e) => {
                    validate_projection_value(value, &e)?;
                }
                J2Command::Evaluation(e) => {
                    if value["locator"] != stage.owner_locator
                        || value["exact"]["claim_id"] != e.claim_id
                        || value["exact"]["claim_revision_id"] != e.finding_revision
                        || value["exact"]["consumer"] != e.consumer
                        || value["exact"]["consumer_revision"] != e.consumer_tree
                        || value["exact"]["decision_scope"] != e.decision_scope
                    {
                        return Err(invalid());
                    }
                }
                _ => return Err(invalid()),
            }
        }
        _ => return Err(invalid()),
    }
    Ok(())
}
fn validate_j2_binding(
    state: &Snapshot,
    intent: &CompletionIntent,
    stage: &StageIntent,
    b: &AdmissionBinding,
) -> Result<()> {
    let mut expected = admission_for(state, intent, &stage.registration_operation_id)?;
    expected.campaign_revision = stage.registration_base_revision.as_str().into();
    expected.claims_request_sha256 = stage.request_sha256.clone();
    if &expected != b || b.digest() != stage.original_admission_sha256 {
        return Err(j2_error("saved J2 admission differs"));
    }
    Ok(())
}
fn validate_projection_value(v: &Value, e: &ProjectionEnvelope) -> Result<()> {
    let raw = text_field(v, "projectionCanonicalJson")?;
    let parsed = cp(&raw)?;
    if v.as_object().is_none_or(|o| o.len() != 4)
        || v["kind"] != "ce_projection"
        || v["requestId"] != e.request_id
        || v["projectionSha256"] != ce(claim_digest(&parsed))?
        || claim_text_field(&parsed, "request_id")? != e.request_id
    {
        return Err(j2_error("projection encoding differs"));
    }
    let revisions = ce(cf(&parsed, "relevant_exact_revisions")?.as_array())?;
    if revisions
        .iter()
        .map(|v| claim_text_field(v, "id"))
        .collect::<Result<Vec<_>>>()?
        != e.revisions
    {
        return Err(j2_error("projection exact set differs"));
    }
    Ok(())
}
pub(crate) fn validate_j2_attempt(state: &Snapshot, attempt: &AttemptRecord) -> Result<()> {
    let Some(intent) = &attempt.completion_intent else {
        return Ok(());
    };
    let stages = intent
        .stages
        .iter()
        .filter(|s| s.kind.is_j2())
        .collect::<Vec<_>>();
    if stages.is_empty() {
        if intent.outcome.is_some()
            || intent
                .consumption_intents
                .iter()
                .any(|c| c.consumed_at_operation_id.is_some())
            || matches!(
                attempt.outcome,
                Some(AttemptOutcome::Completed | AttemptOutcome::FailedWithResult)
            )
        {
            return Err(j2_error("outcome registration absent"));
        }
        return Ok(());
    }
    if intent.consumption_intents.len() != 2 {
        return Err(j2_error("J2 consumption pair incomplete"));
    }
    let m = manifest(intent)?;
    for kind in [
        StageKind::InitialOutcome,
        StageKind::ConsumptionBuilder,
        StageKind::ConsumptionCampaign,
    ] {
        if stages.iter().filter(|s| s.kind == kind).count() != 1 {
            return Err(j2_error("J2 singleton differs"));
        }
    }
    if stages
        .iter()
        .filter(|s| s.kind == StageKind::FindingProjection)
        .count()
        > 1
    {
        return Err(j2_error("duplicate initial projection"));
    }
    let expected_commands = if established(intent)? {
        finding_envelopes(&m)?
    } else {
        vec![]
    };
    let expected = expected_commands.len();
    for (stage, command) in stages
        .iter()
        .filter(|s| s.kind == StageKind::Finding)
        .zip(&expected_commands)
    {
        if stage.command != encode(&J2Command::Finding(command.clone()))? {
            return Err(j2_error("ordered finding command set differs"));
        }
    }
    if stages
        .iter()
        .filter(|s| s.kind == StageKind::Finding)
        .count()
        != expected
    {
        return Err(j2_error("complete finding set differs"));
    }
    if let Some(outcome) = &intent.outcome {
        let projection = saved_projection(intent)?;
        if let Some(v) = &projection {
            if expected == 0 {
                if v != &empty_projection(intent)? {
                    return Err(j2_error("empty witness differs"));
                }
            } else if Some(v)
                != stage_by_kind(intent, StageKind::FindingProjection)
                    .map(checked_ref)
                    .transpose()?
                    .as_ref()
            {
                return Err(j2_error("publication projection differs"));
            }
        }
        let view = build_view(state, intent, &m, projection)?;
        if encode(outcome)? != encode(&stored_outcome(&view)?)? {
            return Err(j2_error("outcome aggregate differs"));
        }
        let usable = view.kind != InitialOutcomeKind::EvidenceUnestablished;
        if !matches!(
            (&attempt.outcome, usable),
            (Some(AttemptOutcome::Completed), true)
                | (Some(AttemptOutcome::FailedWithResult), false)
        ) || intent.consumption_intents[0].consumed_at_operation_id
            != usable.then(|| view.initial_publication_operation_id.clone())
            || intent.consumption_intents[1]
                .consumed_at_operation_id
                .is_some()
        {
            return Err(j2_error("outcome consumption or attempt differs"));
        }
    } else if attempt.outcome.is_some()
        || intent
            .consumption_intents
            .iter()
            .any(|c| c.consumed_at_operation_id.is_some())
    {
        return Err(j2_error("premature outcome consumption"));
    }
    Ok(())
}
pub(crate) fn validate_j2_evaluations(state: &Snapshot) -> Result<()> {
    let mut ids = std::collections::BTreeSet::new();
    let mut revisions = std::collections::BTreeSet::new();
    for record in &state.progress.evaluations {
        if !ids.insert(&record.operation_id)
            || !revisions.insert((&record.finding_id, &record.finding_revision))
        {
            return Err(j2_error("duplicate finding evaluation"));
        }
        let stage = state
            .progress
            .attempts
            .iter()
            .filter_map(|a| a.completion_intent.as_ref())
            .flat_map(|i| &i.stages)
            .find(|s| s.stage_id == format!("{}:reliance", record.operation_id))
            .ok_or_else(|| j2_error("evaluation stage absent"))?;
        let J2Command::Evaluation(e) = decode(&stage.command)? else {
            return Err(j2_error("evaluation command differs"));
        };
        if e.claim_id != record.finding_id
            || e.finding_revision != record.finding_revision
            || e.valid != record.valid
            || e.consumer_tree != record.consumer_tree
            || e.decision_scope != record.decision_scope
            || record.registration_operation_id != record.operation_id
        {
            return Err(j2_error("evaluation record differs"));
        }
        match record.publication_state.as_str() {
            "registered" => {
                if record.reliance != Value::Null || record.projection != Value::Null {
                    return Err(j2_error("uncommitted evaluation payload differs"));
                }
            }
            "committed" => {
                if record.reliance != checked_ref(stage)? {
                    return Err(j2_error("evaluation reliance differs"));
                }
                let projection = state
                    .progress
                    .attempts
                    .iter()
                    .filter_map(|a| a.completion_intent.as_ref())
                    .flat_map(|i| &i.stages)
                    .find(|s| s.stage_id == format!("{}:projection", record.operation_id))
                    .ok_or_else(|| j2_error("evaluation projection absent"))?;
                if record.projection != checked_ref(projection)? {
                    return Err(j2_error("evaluation projection differs"));
                }
            }
            _ => return Err(j2_error("evaluation publication state invalid")),
        }
    }
    Ok(())
}
impl CampaignInner {
    fn check_j2_admission(&self, b: &AdmissionBinding) -> Result<()> {
        let state = self
            .store
            .read(&self.config.identity)?
            .ok_or_else(|| j2_error("campaign absent"))?;
        if state.remediation.as_ref().is_some_and(|progress| {
            progress
                .rounds
                .iter()
                .any(|round| round.request.request_digest == b.prepared_request_sha256)
        }) {
            return self.check_remediation_finding_admission(&state, b);
        }
        let intent = state
            .progress
            .attempts
            .iter()
            .filter_map(|a| a.completion_intent.as_ref())
            .find(|i| i.obligation_id == b.obligation_id)
            .ok_or_else(|| j2_error("J2 intent absent"))?;
        if !established(intent)? {
            return Err(j2_error("J2 requires established pair"));
        }
        let m = manifest(intent)?;
        let selection = self
            .completion_selection
            .as_ref()
            .ok_or_else(|| j2_error("completion authority not installed"))?;
        if m.selection.as_ref() != Some(&selection.record)
            || self.initial_join.as_ref().is_none_or(|j| {
                j.digest != intent.owner_selection_digest || j.epoch != self.store.epoch
            })
        {
            return Err(j2_error("J2 installed authority differs"));
        }
        let stage = intent
            .stages
            .iter()
            .find(|s| {
                s.kind.is_j2()
                    && s.original_admission_sha256 == b.digest()
                    && s.request_sha256 == b.claims_request_sha256
            })
            .ok_or_else(|| j2_error("exact J2 request not registered"))?;
        validate_j2_stage(&state, intent, stage)?;
        if binding_for(stage)? != *b {
            return Err(j2_error("original J2 admission changed"));
        }
        let receipt = self
            .store
            .receipt(&stage.registration_operation_id)?
            .ok_or_else(|| j2_error("J2 registration receipt absent"))?;
        if receipt.kind != j2_registration_kind(stage.kind)
            || receipt.prior_revision.as_ref() != Some(&stage.registration_base_revision)
            || receipt.identity != state.identity
            || receipt.root_id != self.store.root_id
        {
            return Err(j2_error("J2 registration receipt differs"));
        }
        let dispatch = state
            .progress
            .attempts
            .iter()
            .find(|a| a.attempt_id == intent.attempt_id)
            .and_then(|a| a.dispatch.as_ref())
            .ok_or_else(|| j2_error("dispatch absent"))?;
        if self
            .store
            .dispatch_slot(&state.identity, &intent.obligation_id)?
            != Some((Some(dispatch.operation_id.clone()), true))
        {
            return Err(j2_error("J2 dispatch slot changed"));
        }
        match stage.kind {
            StageKind::Finding => {
                if intent.outcome.is_some() {
                    return Err(j2_error("initial publication already committed"));
                }
            }
            StageKind::FindingProjection => {}
            StageKind::EvaluationReliance | StageKind::EvaluationProjection => {
                if !matches!(
                    intent.outcome,
                    Some(InitialOutcome::Reported { .. } | InitialOutcome::AwaitingBuilder { .. })
                ) {
                    return Err(j2_error("evaluation has no usable initial outcome"));
                }
                let op = stage.registration_operation_id.clone();
                if !state.progress.evaluations.iter().any(|e| {
                    e.operation_id == op
                        && (e.publication_state == "registered"
                            || stage.kind == StageKind::EvaluationProjection
                                && e.publication_state == "committed")
                }) {
                    return Err(j2_error("evaluation no longer pending"));
                }
            }
            _ => return Err(j2_error("not an admitted J2 stage")),
        }
        Ok(())
    }
}
fn evaluation_locator(
    gate: &CampaignInner,
    joined: &InitialCompletionLocator,
    operation: &str,
    receipt: &OperationReceipt,
) -> BuilderEvaluationLocator {
    BuilderEvaluationLocator {
        joined: joined.clone(),
        evaluation_operation_id: operation.into(),
        registration_request_sha256: receipt.request_digest.clone(),
        original_registration: gate.locator(
            &joined.identity,
            operation,
            "prepare_builder_evaluation",
            receipt.request_digest.clone(),
            receipt.prior_revision.clone(),
        ),
    }
}
impl Campaign {
    pub fn prepare_builder_evaluation(
        &mut self,
        identity: &CampaignIdentity,
        expected: &CampaignRevision,
        joined: &InitialCompletionLocator,
        operation_id: &str,
        decision: BuilderEvaluationDecision,
        owners: &mut InitialCompletionOwners<'_>,
    ) -> Result<Effect<BuilderEvaluationPrepared>> {
        require_text(operation_id, "evaluation operation")?;
        if identity != &joined.identity {
            return Err(j2_error("evaluation identity differs"));
        }
        if let Some(locator) =
            self.locate_builder_evaluation(identity, &joined.obligation_id, operation_id)?
        {
            let gate = self.owner()?;
            let state = gate
                .store
                .read(identity)?
                .ok_or_else(|| j2_error("campaign absent"))?;
            let saved = state
                .progress
                .evaluations
                .iter()
                .find(|e| e.operation_id == operation_id)
                .ok_or_else(|| j2_error("evaluation absent"))?;
            if saved.finding_id != decision.claim_id.as_str()
                || saved.finding_revision != decision.finding_revision.as_str()
                || saved.valid != decision.valid
                || encode(&locator.joined)? != encode(joined)?
            {
                return Err(j2_error("evaluation operation has different decision"));
            }
            let receipt = gate
                .store
                .receipt(operation_id)?
                .ok_or_else(|| j2_error("evaluation registration absent"))?;
            return Ok(Effect::Replayed {
                value: BuilderEvaluationPrepared {
                    snapshot: state,
                    locator,
                },
                receipt,
            });
        }
        self.owner()?.current(identity, expected)?;
        if self
            .owner()?
            .store
            .read(identity)?
            .as_ref()
            .is_some_and(|state| {
                state
                    .remediation
                    .as_ref()
                    .is_some_and(|progress| progress.current_generation > 0)
            })
        {
            return Err(j2_error("initial generation is no longer current"));
        }
        let (_, intent) = validate_completion_locator(self, joined)?;
        let m = manifest(&intent)?;
        let checked = check_initial(self, joined, &m.operation_id, owners)?;
        if !matches!(
            checked.intent.outcome,
            Some(InitialOutcome::Reported { .. } | InitialOutcome::AwaitingBuilder { .. })
        ) {
            return Err(j2_error("evaluation requires usable publication"));
        }
        verify_committed_outcome_owners(&checked.state, &checked.intent, owners)?;
        let selection = checked
            .manifest
            .selection
            .as_ref()
            .ok_or_else(|| j2_error("builder authority absent"))?;
        let finding = checked
            .intent
            .stages
            .iter()
            .filter(|s| s.kind == StageKind::Finding)
            .map(checked_ref)
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .find(|f| {
                f["claimId"] == decision.claim_id.as_str()
                    && f["revision"] == decision.finding_revision.as_str()
            })
            .ok_or_else(|| j2_error("evaluation is not an exact member finding"))?;
        if checked.state.progress.evaluations.iter().any(|e| {
            e.finding_id == decision.claim_id.as_str()
                && e.finding_revision == decision.finding_revision.as_str()
        }) {
            return Err(j2_error("finding already has a registered disposition"));
        }
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
        let mut command = reliance_command(
            &envelope,
            admission_for(&checked.state, &checked.intent, operation_id)?,
        )?;
        command.admission.claims_request_sha256 =
            ce(claim_evidence::reliance_request_sha256(&command))?;
        let locator = ce(owners.claims.claims.reliance_locator(&command))?;
        if locator.grant_sha256 != selection.reliance_grant_sha256
            || locator.root != selection.root.owner()
        {
            return Err(j2_error("builder reliance grant differs"));
        }
        let stage = make_stage(
            &checked.intent,
            &checked.state,
            format!("{operation_id}:reliance"),
            StageKind::EvaluationReliance,
            operation_id,
            J2Command::Evaluation(envelope),
            Some(command.admission),
            encode(&J2Locator::from(&locator))?,
        )?;
        let revisions = checked
            .intent
            .stages
            .iter()
            .filter(|s| s.kind == StageKind::Finding)
            .map(|s| text_field(&checked_ref(s)?, "revision"))
            .collect::<Result<_>>()?;
        let projection_envelope = projection_envelope(
            &checked.manifest,
            revisions,
            format!("{operation_id}:projection"),
        )?;
        let mut projection = projection_command(
            &projection_envelope,
            admission_for(&checked.state, &checked.intent, operation_id)?,
        )?;
        projection.admission.claims_request_sha256 =
            ce(claim_evidence::projection_request_sha256(&projection))?;
        let projection_stage = make_stage(
            &checked.intent,
            &checked.state,
            projection_envelope.request_id.clone(),
            StageKind::EvaluationProjection,
            operation_id,
            J2Command::Projection(projection_envelope),
            Some(projection.admission),
            encode(&selection.root)?,
        )?;
        let record = EvaluationRecord {
            operation_id: operation_id.into(),
            registration_operation_id: operation_id.into(),
            finding_id: text_field(&finding, "claimId")?,
            finding_revision: text_field(&finding, "revision")?,
            consumer_tree: selection.consumer_tree.clone(),
            decision_scope: selection.decision_scope.clone(),
            valid: decision.valid,
            reliance: Value::Null,
            projection: Value::Null,
            publication_state: "registered".into(),
        };
        let digest = campaign_digest(
            &json!({"identity":identity,"expectedRevision":expected,"record":record,"stage":stage,"projectionStage":projection_stage}),
        )?;
        let mut next = checked.state;
        current_intent_mut(
            &mut next,
            &joined.obligation_id,
            &joined.completion_operation_id,
        )?
        .stages
        .extend([stage, projection_stage]);
        next.progress.evaluations.push(record);
        next.revision = state_revision(&next)?;
        preflight_snapshot(&next)?;
        if serde_json::to_vec(&next)
            .map_err(|e| j2_error(&e.to_string()))?
            .len()
            > 700_000
        {
            return Err(CampaignError::Contract(
                "evaluation capacity preflight".into(),
            ));
        }
        let mut gate = self.owner()?;
        gate.current(identity, expected)?;
        let receipt = gate.receipt(
            &next,
            Some(expected.clone()),
            operation_id,
            "prepare_builder_evaluation",
            digest.clone(),
        );
        let locator = evaluation_locator(&gate, joined, operation_id, &receipt);
        Ok(
            match gate.put(
                next,
                Some(expected.clone()),
                operation_id,
                "prepare_builder_evaluation",
                digest,
                false,
                None,
            ) {
                Effect::Applied { value, receipt } => {
                    crate::fault_cut("j2_after_evaluation_registration");
                    Effect::Applied {
                        value: BuilderEvaluationPrepared {
                            snapshot: value,
                            locator,
                        },
                        receipt,
                    }
                }
                Effect::Replayed { value, receipt } => Effect::Replayed {
                    value: BuilderEvaluationPrepared {
                        snapshot: value,
                        locator,
                    },
                    receipt,
                },
                Effect::NoEffect(e) => Effect::NoEffect(e),
                Effect::OutcomeUnknown(l) => Effect::OutcomeUnknown(l),
            },
        )
    }
    pub fn locate_builder_evaluation(
        &self,
        identity: &CampaignIdentity,
        obligation_id: &str,
        evaluation_operation_id: &str,
    ) -> Result<Option<BuilderEvaluationLocator>> {
        let gate = self.owner()?;
        gate.check_identity(identity)?;
        let Some(state) = gate.store.read(identity)? else {
            return Ok(None);
        };
        if !state
            .obligations
            .iter()
            .any(|o| o.obligation_id == obligation_id)
        {
            return Err(j2_error("evaluation obligation absent"));
        }
        let Some(receipt) = gate.store.receipt(evaluation_operation_id)? else {
            return Ok(None);
        };
        let record = state
            .progress
            .evaluations
            .iter()
            .find(|e| e.operation_id == evaluation_operation_id)
            .ok_or_else(|| j2_error("evaluation registration conflicts"))?;
        let intent = state
            .progress
            .attempts
            .iter()
            .filter_map(|a| a.completion_intent.as_ref())
            .find(|i| i.obligation_id == obligation_id)
            .ok_or_else(|| j2_error("evaluation completion intent absent"))?;
        if receipt.kind != "prepare_builder_evaluation"
            || record.registration_operation_id != evaluation_operation_id
        {
            return Err(j2_error("evaluation registration differs"));
        }
        Ok(Some(evaluation_locator(
            &gate,
            &manifest(intent)?.joined,
            evaluation_operation_id,
            &receipt,
        )))
    }
    pub fn complete_builder_evaluation(
        &mut self,
        identity: &CampaignIdentity,
        expected: &CampaignRevision,
        locator: &BuilderEvaluationLocator,
        owners: &mut InitialCompletionOwners<'_>,
    ) -> Result<BuilderEvaluationProgress> {
        if identity != &locator.joined.identity {
            return Err(j2_error("evaluation identity differs"));
        }
        self.owner()?.current(identity, expected)?;
        self.recover_builder_evaluation(locator, owners)
    }
    pub fn recover_builder_evaluation(
        &mut self,
        locator: &BuilderEvaluationLocator,
        owners: &mut InitialCompletionOwners<'_>,
    ) -> Result<BuilderEvaluationProgress> {
        let found = self
            .locate_builder_evaluation(
                &locator.joined.identity,
                &locator.joined.obligation_id,
                &locator.evaluation_operation_id,
            )?
            .ok_or_else(|| j2_error("evaluation registration absent"))?;
        if encode(&found)? != encode(locator)? {
            return Err(j2_error("evaluation locator differs"));
        }
        drive_evaluation(self, locator, owners)
    }
}
fn drive_evaluation(
    c: &mut Campaign,
    l: &BuilderEvaluationLocator,
    owners: &mut InitialCompletionOwners<'_>,
) -> Result<BuilderEvaluationProgress> {
    let (mut last, intent) = validate_completion_locator(c, &l.joined)?;
    let m = manifest(&intent)?;
    let mut newly = vec![];
    let mut active = StageKind::EvaluationReliance;
    let run = (|| -> Result<(BuilderEvaluationView, OperationReceipt)> {
        let checked = check_initial(c, &l.joined, &m.operation_id, owners)?;
        last = checked.state;
        // Every aggregate uses all committed dispositions, including those
        // belonging to another concurrently prepared evaluation.
        verify_committed_outcome_owners(&last, &checked.intent, owners)?;
        let record = last
            .progress
            .evaluations
            .iter()
            .find(|e| e.operation_id == l.evaluation_operation_id)
            .ok_or_else(|| j2_error("evaluation absent"))?
            .clone();
        if record.publication_state == "committed" {
            let intent = current_intent(
                &last,
                &l.joined.obligation_id,
                &l.joined.completion_operation_id,
            )?;
            let view = evaluation_view(&last, intent, &m, &record)?;
            let receipt = c
                .owner()?
                .store
                .receipt(&format!("{}:publish", record.operation_id))?
                .ok_or_else(|| j2_error("evaluation publication receipt absent"))?;
            return Ok((view, receipt));
        }
        let stage = checked
            .intent
            .stages
            .iter()
            .find(|s| s.stage_id == format!("{}:reliance", l.evaluation_operation_id))
            .ok_or_else(|| j2_error("reliance stage absent"))?;
        let J2Command::Evaluation(e) = decode(&stage.command)? else {
            return Err(j2_error("evaluation command differs"));
        };
        let command = reliance_command(&e, binding_for(stage)?)?;
        let locator = decode::<J2Locator>(&stage.owner_locator)?.owner()?;
        if ce(owners.claims.claims.reliance_locator(&command))? != locator {
            return Err(j2_error("reliance locator changed"));
        }
        let receipt = match owners.claims.claims.reconcile_reliance(&locator) {
            RelianceReconciliation::Committed(r) => *r,
            RelianceReconciliation::Absent => match owners.claims.claims.record_reliance(command) {
                Ok(ReliancePublication::Applied(r) | ReliancePublication::Replayed(r)) => r,
                Err(claim_evidence::WriteError::NoEffect(e)) => {
                    return Err(j2_error(&e.to_string()));
                }
                Err(claim_evidence::WriteError::OutcomeUnknown(locator)) => {
                    return Err(CampaignError::Io(format!(
                        "reliance publication uncertain: {locator:?}"
                    )));
                }
            },
            RelianceReconciliation::Conflicting => {
                return Err(j2_error("reliance owner record conflicts"));
            }
            RelianceReconciliation::Unresolved(reason) => return Err(CampaignError::Io(reason)),
        };
        let reliance = json!({"exact":value_from_claim(&receipt.exact.to_js_value())?,"locator":encode(&J2Locator::from(&receipt.locator))?});
        newly.push(reliance.clone());
        crate::fault_cut("j2_after_reliance_commit");
        last = j2_progress_commit(
            c,
            &l.joined,
            &stage.stage_id,
            checked_progress(reliance.clone())?,
        )?;
        active = StageKind::EvaluationProjection;
        let intent = current_intent(
            &last,
            &l.joined.obligation_id,
            &l.joined.completion_operation_id,
        )?;
        let revisions = intent
            .stages
            .iter()
            .filter(|s| s.kind == StageKind::Finding)
            .map(|s| text_field(&checked_ref(s)?, "revision"))
            .collect::<Result<_>>()?;
        let envelope =
            projection_envelope(&m, revisions, format!("{}:projection", record.operation_id))?;
        let projection_stage = register_projection(
            c,
            &l.joined,
            &m,
            StageKind::EvaluationProjection,
            &format!("{}:projection-register", record.operation_id),
            envelope.clone(),
        )?;
        let projection = ce(owners
            .claims
            .claims
            .project_exact_revisions(&projection_command(
                &envelope,
                binding_for(&projection_stage)?,
            )?))?;
        if !projection.relevant_exact_revisions().iter().any(|r| {
            r.exact.claim_id.as_str() == record.finding_id
                && r.exact.revision_id.as_str() == record.finding_revision
                && r.current_reliances.contains(&receipt.exact)
        }) {
            return Err(j2_error("fresh projection lacks exact current reliance"));
        }
        let projected = projection_value(&projection)?;
        newly.push(projected.clone());
        crate::fault_cut("j2_after_evaluation_projection_read");
        last = j2_progress_commit(
            c,
            &l.joined,
            &projection_stage.stage_id,
            checked_progress(projected.clone())?,
        )?;
        ce(owners.claims.claims.verify_projection_fresh(&projection))?;
        active = StageKind::EvaluationCommit;
        let mut next = last.clone();
        let evaluation = next
            .progress
            .evaluations
            .iter_mut()
            .find(|r| r.operation_id == record.operation_id)
            .unwrap();
        evaluation.reliance = reliance;
        evaluation.projection = projected;
        evaluation.publication_state = "committed".into();
        let committed = evaluation.clone();
        let intent = current_intent(
            &next,
            &l.joined.obligation_id,
            &l.joined.completion_operation_id,
        )?;
        // Recheck original owner admissions and current membership after the
        // downstream effects, before publishing an aggregate that uses them.
        // Keep this outside the campaign lock: projection admission reads it.
        verify_committed_outcome_owners(&next, intent, owners)?;
        let view = evaluation_view(&next, intent, &m, &committed)?;
        current_intent_mut(
            &mut next,
            &l.joined.obligation_id,
            &l.joined.completion_operation_id,
        )?
        .outcome = Some(stored_outcome(&view.outcome)?);
        next.revision = state_revision(&next)?;
        preflight_snapshot(&next)?;
        let digest = campaign_digest(&encode(&view)?)?;
        let mut gate = c.owner()?;
        let current = gate.current(&l.joined.identity, &last.revision)?;
        crate::fault_cut("j2_before_evaluation_commit");
        match gate.put(
            next,
            Some(current.revision),
            &format!("{}:publish", record.operation_id),
            "publish_builder_evaluation",
            digest,
            false,
            None,
        ) {
            Effect::Applied { value, receipt } | Effect::Replayed { value, receipt } => {
                last = value;
                newly.push(encode(&receipt)?);
                crate::fault_cut("j2_after_evaluation_commit");
                Ok((view, receipt))
            }
            Effect::NoEffect(e) => Err(j2_error(&e)),
            Effect::OutcomeUnknown(x) => Err(CampaignError::Io(format!(
                "evaluation publication uncertain: {x:?}"
            ))),
        }
    })();
    let stages = current_intent(
        &last,
        &l.joined.obligation_id,
        &l.joined.completion_operation_id,
    )?
    .stages
    .clone();
    Ok(BuilderEvaluationProgress {
        locator: l.clone(),
        last_checked_revision: last.revision,
        stages,
        newly_observed_receipts: newly,
        state: match run {
            Ok((view, receipt)) => BuilderEvaluationState::Committed { view, receipt },
            Err(CampaignError::Unsupported(reason)) => BuilderEvaluationState::Pending {
                stage: active,
                reason,
            },
            Err(CampaignError::Conflict(reason)) => BuilderEvaluationState::Conflicting {
                stage: active,
                reason,
            },
            Err(e) => BuilderEvaluationState::Unresolved {
                stage: active,
                owner_locator: encode(l)?,
                reason: e.to_string(),
            },
        },
    })
}
fn evaluation_view(
    state: &Snapshot,
    intent: &CompletionIntent,
    m: &OutcomeManifest,
    r: &EvaluationRecord,
) -> Result<BuilderEvaluationView> {
    Ok(BuilderEvaluationView {
        operation_id: r.operation_id.clone(),
        claim_id: r.finding_id.clone(),
        finding_revision: r.finding_revision.clone(),
        valid: r.valid,
        consumer_tree: r.consumer_tree.clone(),
        decision_scope: r.decision_scope.clone(),
        reliance: r.reliance.clone(),
        projection: r.projection.clone(),
        outcome: build_view(state, intent, m, saved_projection(intent)?)?,
    })
}

fn verify_committed_outcome_owners(
    state: &Snapshot,
    intent: &CompletionIntent,
    owners: &InitialCompletionOwners<'_>,
) -> Result<()> {
    for stage in intent
        .stages
        .iter()
        .filter(|s| s.kind == StageKind::Finding)
    {
        let J2Command::Finding(envelope) = decode(&stage.command)? else {
            return Err(j2_error("finding command differs"));
        };
        let locator = decode::<J2Locator>(&stage.owner_locator)?.owner()?;
        match owners.claims.claims.reconcile(&locator) {
            Reconciliation::Committed(receipt)
                if finding_receipt(&receipt, &envelope)? == checked_ref(stage)? => {}
            Reconciliation::Absent => {
                return Err(CampaignError::Unsupported(
                    "committed finding owner record absent".into(),
                ));
            }
            Reconciliation::Unresolved(reason) => return Err(CampaignError::Io(reason)),
            _ => return Err(j2_error("committed finding owner receipt differs")),
        }
    }
    let mut reliances = vec![];
    for record in state
        .progress
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
            .ok_or_else(|| j2_error("committed evaluation reliance stage absent"))?;
        let J2Command::Evaluation(envelope) = decode(&stage.command)? else {
            return Err(j2_error("evaluation command differs"));
        };
        let locator = decode::<J2Locator>(&stage.owner_locator)?.owner()?;
        let command = reliance_command(&envelope, binding_for(stage)?)?;
        if ce(owners.claims.claims.reliance_locator(&command))? != locator {
            return Err(j2_error("committed reliance owner locator differs"));
        }
        let receipt = match owners.claims.claims.reconcile_reliance(&locator) {
            RelianceReconciliation::Committed(receipt) => receipt,
            RelianceReconciliation::Absent => {
                return Err(CampaignError::Unsupported(
                    "committed reliance owner record absent".into(),
                ));
            }
            RelianceReconciliation::Unresolved(reason) => return Err(CampaignError::Io(reason)),
            RelianceReconciliation::Conflicting => {
                return Err(j2_error("committed reliance owner record differs"));
            }
        };
        let found = json!({"exact":value_from_claim(&receipt.exact.to_js_value())?,
            "locator":encode(&J2Locator::from(&receipt.locator))?});
        if found != record.reliance || found != checked_ref(stage)? {
            return Err(j2_error("committed reliance owner receipt differs"));
        }
        reliances.push(receipt.exact);
    }
    if let Some(stage) = stage_by_kind(intent, StageKind::FindingProjection) {
        let J2Command::Projection(envelope) = decode(&stage.command)? else {
            return Err(j2_error("projection command differs"));
        };
        let projected = ce(owners
            .claims
            .claims
            .project_exact_revisions(&projection_command(&envelope, binding_for(stage)?)?))?;
        for revision in projected.relevant_exact_revisions() {
            if !intent
                .stages
                .iter()
                .filter(|s| s.kind == StageKind::Finding)
                .map(checked_ref)
                .collect::<Result<Vec<_>>>()?
                .iter()
                .any(|f| {
                    f["claimId"] == revision.exact.claim_id.as_str()
                        && f["revision"] == revision.exact.revision_id.as_str()
                        && f["revisionSha256"] == revision.exact.revision_sha256
                })
            {
                return Err(j2_error("committed projection exact revision differs"));
            }
        }
        for reliance in &reliances {
            if !projected.relevant_exact_revisions().iter().any(|revision| {
                revision.exact == reliance.finding && revision.current_reliances.contains(reliance)
            }) {
                return Err(j2_error(
                    "current projection lacks committed exact reliance",
                ));
            }
        }
        ce(owners.claims.claims.verify_projection_fresh(&projected))?;
    } else if !reliances.is_empty() {
        return Err(j2_error("committed evaluations lack finding projection"));
    }
    Ok(())
}
pub(crate) fn validate_j2_registration(
    receipt: &OperationReceipt,
    registered: &Snapshot,
) -> Result<()> {
    let stages: Vec<_> = registered
        .progress
        .attempts
        .iter()
        .filter_map(|a| a.completion_intent.as_ref())
        .flat_map(|i| &i.stages)
        .filter(|s| s.registration_operation_id == receipt.operation_id)
        .collect();
    let digest = match receipt.kind.as_str() {
        "register_initial_outcome" => campaign_digest(
            &json!({"identity":registered.identity,"expectedRevision":receipt.prior_revision,"operationId":receipt.operation_id,"stages":stages}),
        )?,
        "register_j2_projection" if stages.len() == 1 => campaign_digest(
            &json!({"identity":registered.identity,"expectedRevision":receipt.prior_revision,"stage":stages[0]}),
        )?,
        "prepare_builder_evaluation" if stages.len() == 2 => {
            let reliance = stages
                .iter()
                .find(|s| s.kind == StageKind::EvaluationReliance)
                .ok_or_else(|| j2_error("registered reliance absent"))?;
            let projection = stages
                .iter()
                .find(|s| s.kind == StageKind::EvaluationProjection)
                .ok_or_else(|| j2_error("registered evaluation projection absent"))?;
            let record = registered
                .progress
                .evaluations
                .iter()
                .find(|e| e.operation_id == receipt.operation_id)
                .ok_or_else(|| j2_error("registered evaluation absent"))?;
            campaign_digest(
                &json!({"identity":registered.identity,"expectedRevision":receipt.prior_revision,"record":record,"stage":reliance,"projectionStage":projection}),
            )?
        }
        _ => return Err(j2_error("J2 registration kind or stage set differs")),
    };
    if digest != receipt.request_digest {
        return Err(j2_error("J2 registration request digest differs"));
    }
    Ok(())
}
pub(crate) fn j2_publication_digest(
    state: &Snapshot,
    operation: &str,
    kind: &str,
) -> Result<String> {
    let intent = state
        .progress
        .attempts
        .iter()
        .filter_map(|a| a.completion_intent.as_ref())
        .find(|i| i.outcome.is_some())
        .ok_or_else(|| j2_error("published outcome absent"))?;
    let m = manifest(intent)?;
    let view = match kind {
        "publish_initial_outcome" if operation == format!("{}:publish", m.operation_id) => {
            encode(&build_view(state, intent, &m, saved_projection(intent)?)?)?
        }
        "publish_builder_evaluation" => {
            let r = state
                .progress
                .evaluations
                .iter()
                .find(|r| operation == format!("{}:publish", r.operation_id))
                .ok_or_else(|| j2_error("published evaluation absent"))?;
            encode(&evaluation_view(state, intent, &m, r)?)?
        }
        _ => return Err(j2_error("publication kind differs")),
    };
    campaign_digest(&view)
}

pub(crate) fn j2_progress_operation(
    id: &str,
    kind: StageKind,
    progress: &StageProgress,
) -> Result<String> {
    if matches!(
        kind,
        StageKind::FindingProjection | StageKind::EvaluationProjection
    ) {
        // Freshness rechecks retain the original registered read identity while
        // attributing each changed readback to its own immutable campaign receipt.
        Ok(format!(
            "{id}:checked:{}",
            campaign_digest(&encode(progress)?)?
        ))
    } else {
        Ok(format!("{id}:checked"))
    }
}
