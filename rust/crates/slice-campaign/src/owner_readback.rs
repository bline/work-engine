// Concrete J1 owner composition. This file is included in application.rs so
// its leases and reducers can use the same private campaign gate.
use crate::completion::{
    CompletionIntent, ConsumptionIntent, ExecutionRefRecord, JoinedReadbacks, ObservationSelection,
    SelectedClaim, StageIntent, StageKind, StageProgress,
};
use claim_evidence::codec::{
    JsValue as ClaimValue, canonical_json as claim_json, digest as claim_digest,
    parse_json as parse_claim_json,
};
use claim_evidence::{
    AdmissionReadRequest, BootstrapFiles as ClaimsBootstrapFiles, CheckedClaimEvidence,
    ClaimsApplication, EstablishmentCommand, ObservationCommand, OpenFiles as ClaimsOpenFiles,
    ProductionPathAccess, ProductionPathAdmissionBinding, ProductionPathAdmissionPort,
    ProductionPathLease, ProductionPathLocator, ProductionPathPublication,
    ProductionPathReconciliation, ProductionPathRequest, ProductionPathStage,
    ProductionPathWriteError, RootIdentity, production_path_request_sha256,
};
use review_episode::Application as EpisodeApplication;
use review_episode::in_process_admission::{EpisodeReadRequest, InProcessReadAdmission};
use review_episode_core::codec::{
    JsString as EpisodeString, JsValue as EpisodeValue, digest as episode_value_digest,
    parse_json as parse_episode_json,
};
use review_episode_core::identity::{Authority, Revision as EpisodeRevision};
use review_episode_core::implementation_review_v1::ReviewResult;
use review_episode_store::NativeRootSelection;
use review_execution_evidence::{
    CheckedExecutionResult, CheckedObservation, EvidenceReaderProfile, ExecutionEvidenceOwner,
    ExecutionEvidenceRef,
};

/// Trusted startup selection. These values are checked against the actual
/// owners; the fields are descriptive and never a request-side authority grant.
#[derive(Clone)]
pub struct InitialJoinSelection {
    pub claims_root: RootIdentity,
    pub production_grant_id: String,
    pub production_grant_sha256: String,
    pub episode_selection: NativeRootSelection,
    pub episode_authority: Authority,
    pub evidence_profile: EvidenceReaderProfile,
    pub evidence_owner: String,
    pub evidence_root_id: String,
    pub evidence_record_profile: String,
}

impl InitialJoinSelection {
    fn digest(&self) -> Result<String> {
        for value in [
            &self.production_grant_id,
            &self.evidence_owner,
            &self.evidence_root_id,
            &self.evidence_record_profile,
        ] {
            require_text(value, "initial join selection")?;
        }
        crate::require_sha(&self.production_grant_sha256, "production grant")?;
        if !self.claims_root.absolute_path.is_absolute()
            || !Path::new(&self.episode_selection.root).is_absolute()
            || self.claims_root.root_id.is_empty()
            || self.claims_root.profile.is_empty()
            || self.episode_authority.writer().generation != 1
            || self.episode_authority.predecessor_revision().is_some()
        {
            return Err(CampaignError::Contract(
                "initial join owner selection invalid".into(),
            ));
        }
        let authority_json =
            review_episode_core::codec::canonical_json(self.episode_authority.value());
        let authority_value: Value = serde_json::from_str(&authority_json)
            .map_err(|e| CampaignError::Contract(e.to_string()))?;
        campaign_digest(&json!({
            "claimsRoot":{"path":self.claims_root.absolute_path,"rootId":self.claims_root.root_id,
                "profile":self.claims_root.profile},
            "productionGrantId":self.production_grant_id,
            "productionGrantSha256":self.production_grant_sha256,
            "episodeSelection":{"root":self.episode_selection.root,
                "executableSha256":self.episode_selection.executable_sha256,
                "selectionDigest":self.episode_selection.selection_digest},
            "episodeAuthority":authority_value,
            "evidenceProfile":format!("{:?}",self.evidence_profile),
            "evidenceOwner":self.evidence_owner,
            "evidenceRootId":self.evidence_root_id,
            "evidenceRecordProfile":self.evidence_record_profile,
        }))
    }
}

#[derive(Clone)]
struct InitialJoinInstallation {
    selection: InitialJoinSelection,
    digest: String,
    evidence: Arc<dyn ExecutionEvidenceOwner>,
    epoch: String,
}

/// The CE3 port is concrete and privately constructed from one campaign open.
/// Its clone cannot install a different evidence reader or owner selection.
#[derive(Clone)]
pub struct CE3CampaignClaimsAdmission {
    inner: Arc<Mutex<CampaignInner>>,
    campaign_root_id: String,
    campaign_epoch: String,
    selection_digest: String,
}

struct CE3Lease<'a> {
    _gate: MutexGuard<'a, CampaignInner>,
    evidence: Arc<dyn ExecutionEvidenceOwner>,
    reference: ExecutionEvidenceRef,
    profile: EvidenceReaderProfile,
    binding: ProductionPathAdmissionBinding,
}

impl FindingLease for CE3Lease<'_> {}
impl FindingAdmissionPort for CE3CampaignClaimsAdmission {
    fn acquire<'a>(
        &'a self,
        _binding: &AdmissionBinding,
    ) -> ClaimResult<Box<dyn FindingLease + 'a>> {
        Err(ClaimError::new(
            "J2 finding/reliance admission is not enabled",
        ))
    }
}
impl ProductionPathLease for CE3Lease<'_> {
    fn read_custody(
        &self,
        observation: Option<&ClaimValue>,
    ) -> ClaimResult<claim_evidence::CustodyEvidence> {
        let checked = self
            .evidence
            .read_result(&self.reference)
            .map_err(|e| ClaimError::new(e.to_string()))?;
        self.profile
            .require_class(checked.evidence_class())
            .map_err(|e| ClaimError::new(e.to_string()))?;
        if checked.reference() != &self.reference
            || checked.claim_sha256() != self.binding.native_result_sha256
        {
            return Err(ClaimError::new(
                "execution evidence changed during CE lease",
            ));
        }
        self.evidence
            .read_custody(&self.reference, &self.binding, observation)
            .map_err(|e| ClaimError::new(e.to_string()))
    }
}
impl ProductionPathAdmissionPort for CE3CampaignClaimsAdmission {
    fn acquire_production_path<'a>(
        &'a self,
        binding: &ProductionPathAdmissionBinding,
        request: &ProductionPathRequest,
    ) -> ClaimResult<Box<dyn ProductionPathLease + 'a>> {
        let gate = self
            .inner
            .lock()
            .map_err(|_| ClaimError::new("campaign gate poisoned"))?;
        let installation = gate
            .initial_join
            .as_ref()
            .ok_or_else(|| ClaimError::new("initial join not installed"))?;
        if gate.store.root_id != self.campaign_root_id
            || gate.store.epoch != self.campaign_epoch
            || installation.epoch != self.campaign_epoch
            || installation.digest != self.selection_digest
        {
            return Err(ClaimError::new("stale or mismatched CE3 campaign port"));
        }
        let reference = gate
            .check_production_path_admission(binding, request)
            .map_err(|e| ClaimError::new(e.to_string()))?;
        Ok(Box::new(CE3Lease {
            evidence: Arc::clone(&installation.evidence),
            profile: installation.selection.evidence_profile,
            reference,
            binding: binding.clone(),
            _gate: gate,
        }))
    }
}

/// This wrapper fixes the *actual* concrete port passed to the real CE owner.
/// It cannot be assembled from a caller-provided claims application.
pub struct InitialClaimsOwner {
    claims: ClaimsApplication<CE3CampaignClaimsAdmission>,
    campaign: Arc<Mutex<CampaignInner>>,
    campaign_epoch: String,
    selection_digest: String,
}
impl InitialClaimsOwner {
    pub fn claims(&self) -> &ClaimsApplication<CE3CampaignClaimsAdmission> {
        &self.claims
    }
    pub fn claims_mut(&mut self) -> &mut ClaimsApplication<CE3CampaignClaimsAdmission> {
        &mut self.claims
    }
}

pub struct InitialCompletionOwners<'a> {
    claims: &'a mut InitialClaimsOwner,
    episode: Option<&'a mut EpisodeApplication<InProcessReadAdmission>>,
}
impl<'a> InitialCompletionOwners<'a> {
    pub fn new(
        claims: &'a mut InitialClaimsOwner,
        episode: Option<&'a mut EpisodeApplication<InProcessReadAdmission>>,
    ) -> Self {
        Self { claims, episode }
    }
}

impl Campaign {
    pub fn claims_admission_with_custody(
        &self,
        selection: InitialJoinSelection,
        evidence: Arc<dyn ExecutionEvidenceOwner>,
    ) -> Result<CE3CampaignClaimsAdmission> {
        let digest = selection.digest()?;
        let mut owner = self.owner()?;
        if owner.store.root_id != self.root_id || owner.store.root != self.root {
            return Err(CampaignError::Conflict("campaign root changed".into()));
        }
        if let Some(existing) = &owner.initial_join {
            if existing.digest != digest || !Arc::ptr_eq(&existing.evidence, &evidence) {
                return Err(CampaignError::Conflict(
                    "initial join already installed differently".into(),
                ));
            }
        } else {
            owner.initial_join = Some(InitialJoinInstallation {
                selection,
                digest: digest.clone(),
                evidence,
                epoch: owner.store.epoch.clone(),
            });
        }
        Ok(CE3CampaignClaimsAdmission {
            inner: Arc::clone(&self.inner),
            campaign_root_id: self.root_id.clone(),
            campaign_epoch: owner.store.epoch.clone(),
            selection_digest: digest,
        })
    }

    pub fn initialize_initial_claims(
        &self,
        files: ClaimsBootstrapFiles,
        selection: InitialJoinSelection,
        evidence: Arc<dyn ExecutionEvidenceOwner>,
    ) -> Result<InitialClaimsOwner> {
        let expected = selection.claims_root.clone();
        let port = self.claims_admission_with_custody(selection, evidence)?;
        let claims = ClaimsApplication::initialize(files, port.clone())
            .map_err(|e| CampaignError::Contract(e.to_string()))?;
        self.wrap_initial_claims(claims, &port, &expected)
    }

    pub fn open_initial_claims(
        &self,
        files: ClaimsOpenFiles,
        selection: InitialJoinSelection,
        evidence: Arc<dyn ExecutionEvidenceOwner>,
    ) -> Result<InitialClaimsOwner> {
        let expected = selection.claims_root.clone();
        let port = self.claims_admission_with_custody(selection, evidence)?;
        let claims = ClaimsApplication::open(files, port.clone())
            .map_err(|e| CampaignError::Contract(e.to_string()))?;
        self.wrap_initial_claims(claims, &port, &expected)
    }

    fn wrap_initial_claims(
        &self,
        claims: ClaimsApplication<CE3CampaignClaimsAdmission>,
        port: &CE3CampaignClaimsAdmission,
        expected: &RootIdentity,
    ) -> Result<InitialClaimsOwner> {
        if claims.root() != expected {
            return Err(CampaignError::Conflict(
                "selected CE root differs from actual owner".into(),
            ));
        }
        Ok(InitialClaimsOwner {
            claims,
            campaign: Arc::clone(&self.inner),
            campaign_epoch: port.campaign_epoch.clone(),
            selection_digest: port.selection_digest.clone(),
        })
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BindingRecord {
    campaign_root_id: String,
    campaign_revision: String,
    campaign_operation_id: String,
    obligation_id: String,
    candidate_digest: String,
    selection_digest: String,
    profile_digest: String,
    prepared_request_sha256: String,
    child_request_sha256: String,
    review_episode_id: String,
    attempt_id: String,
    native_result_sha256: String,
    episode_revision: Option<String>,
    session_id: Option<String>,
    stage: String,
    access: String,
}
impl From<&ProductionPathAdmissionBinding> for BindingRecord {
    fn from(v: &ProductionPathAdmissionBinding) -> Self {
        Self {
            campaign_root_id: v.campaign_root_id.clone(),
            campaign_revision: v.campaign_revision.clone(),
            campaign_operation_id: v.campaign_operation_id.clone(),
            obligation_id: v.obligation_id.clone(),
            candidate_digest: v.candidate_digest.clone(),
            selection_digest: v.selection_digest.clone(),
            profile_digest: v.profile_digest.clone(),
            prepared_request_sha256: v.prepared_request_sha256.clone(),
            child_request_sha256: v.child_request_sha256.clone(),
            review_episode_id: v.review_episode_id.clone(),
            attempt_id: v.attempt_id.clone(),
            native_result_sha256: v.native_result_sha256.clone(),
            episode_revision: v.episode_revision.clone(),
            session_id: v.session_id.clone(),
            stage: match v.stage {
                ProductionPathStage::RecordObservation => "record_observation",
                ProductionPathStage::EstablishClaim => "establish_claim",
                ProductionPathStage::ReadAdmission => "read_admission",
            }
            .into(),
            access: match v.access {
                ProductionPathAccess::Original => "original",
                ProductionPathAccess::RecoverExact => "recover_exact",
            }
            .into(),
        }
    }
}
impl BindingRecord {
    fn checked(self) -> Result<ProductionPathAdmissionBinding> {
        let stage = match self.stage.as_str() {
            "record_observation" => ProductionPathStage::RecordObservation,
            "establish_claim" => ProductionPathStage::EstablishClaim,
            "read_admission" => ProductionPathStage::ReadAdmission,
            _ => return Err(CampaignError::Integrity("stage binding tag invalid".into())),
        };
        let access = match self.access.as_str() {
            "original" => ProductionPathAccess::Original,
            "recover_exact" => ProductionPathAccess::RecoverExact,
            _ => return Err(CampaignError::Integrity("stage access tag invalid".into())),
        };
        let v = ProductionPathAdmissionBinding {
            campaign_root_id: self.campaign_root_id,
            campaign_revision: self.campaign_revision,
            campaign_operation_id: self.campaign_operation_id,
            obligation_id: self.obligation_id,
            candidate_digest: self.candidate_digest,
            selection_digest: self.selection_digest,
            profile_digest: self.profile_digest,
            prepared_request_sha256: self.prepared_request_sha256,
            child_request_sha256: self.child_request_sha256,
            review_episode_id: self.review_episode_id,
            attempt_id: self.attempt_id,
            native_result_sha256: self.native_result_sha256,
            episode_revision: self.episode_revision,
            session_id: self.session_id,
            stage,
            access,
        };
        v.validate()
            .map_err(|e| CampaignError::Integrity(e.to_string()))?;
        Ok(v)
    }
}
fn binding_value(v: &ProductionPathAdmissionBinding) -> Value {
    serde_json::to_value(BindingRecord::from(v)).expect("binding adapter is serializable")
}
fn decode_binding(v: &Value) -> Result<ProductionPathAdmissionBinding> {
    serde_json::from_value::<BindingRecord>(v.clone())
        .map_err(|e| CampaignError::Integrity(e.to_string()))?
        .checked()
}
fn value_from_claim(v: &ClaimValue) -> Result<Value> {
    let raw = claim_json(v).map_err(|e| CampaignError::Contract(e.to_string()))?;
    serde_json::from_str(&raw).map_err(|e| CampaignError::Contract(e.to_string()))
}
fn claim_from_value(v: &Value) -> Result<ClaimValue> {
    parse_claim_json(
        &serde_json::to_string(v).map_err(|e| CampaignError::Integrity(e.to_string()))?,
    )
    .map_err(|e| CampaignError::Integrity(e.to_string()))
}
fn text_field(v: &Value, field: &str) -> Result<String> {
    v.get(field)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| CampaignError::Integrity(format!("{field} absent")))
}
fn claim_text_field(v: &ClaimValue, field: &str) -> Result<String> {
    v.get(field)
        .ok_or_else(|| CampaignError::Integrity(format!("{field} absent")))?
        .as_text()
        .map_err(|e| CampaignError::Integrity(e.to_string()))?
        .to_string_checked()
        .map_err(|e| CampaignError::Contract(e.to_string()))
}
fn command_value(request: &ProductionPathRequest) -> Result<Value> {
    Ok(match request {
        ProductionPathRequest::RecordObservation(c) => json!({"grantId":c.grant_id,
            "observationCanonicalJson":claim_json(&c.observation)
                .map_err(|e|CampaignError::Contract(e.to_string()))?}),
        ProductionPathRequest::EstablishClaim(c) => json!({"grantId":c.grant_id,
            "operationId":c.operation_id,"claim":value_from_claim(&c.claim)?,
            "observationId":c.observation_id}),
        ProductionPathRequest::ReadAdmission(c) => json!({"grantId":c.grant_id,
            "claim":value_from_claim(&c.claim)?,"establishmentId":c.establishment_id,
            "establishmentSha256":c.establishment_sha256,
            "establishmentRequestSha256":c.establishment_request_sha256,
            "establishmentAdmissionSha256":c.establishment_admission_sha256,
            "observationId":c.observation_id,"observationSha256":c.observation_sha256}),
    })
}
fn stage_request(
    stage: &StageIntent,
    access: ProductionPathAccess,
) -> Result<ProductionPathRequest> {
    let mut binding = decode_binding(&stage.original_admission)?;
    if binding.access != ProductionPathAccess::Original
        || binding.original_digest() != stage.original_admission_sha256
    {
        return Err(CampaignError::Integrity(
            "original CE binding differs".into(),
        ));
    }
    binding.access = access;
    let c = &stage.command;
    let grant_id = text_field(c, "grantId")?;
    let request = match stage.kind {
        StageKind::Observation => ProductionPathRequest::RecordObservation(ObservationCommand {
            grant_id,
            admission: binding,
            observation: parse_claim_json(&text_field(c, "observationCanonicalJson")?)
                .map_err(|e| CampaignError::Integrity(e.to_string()))?,
        }),
        StageKind::EstablishmentBuilder | StageKind::EstablishmentCampaign => {
            ProductionPathRequest::EstablishClaim(EstablishmentCommand {
                grant_id,
                admission: binding,
                operation_id: text_field(c, "operationId")?,
                claim: claim_from_value(
                    c.get("claim")
                        .ok_or_else(|| CampaignError::Integrity("claim absent".into()))?,
                )?,
                observation_id: c
                    .get("observationId")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            })
        }
        StageKind::AdmissionBuilder | StageKind::AdmissionCampaign => {
            ProductionPathRequest::ReadAdmission(AdmissionReadRequest {
                grant_id,
                admission: binding,
                claim: claim_from_value(
                    c.get("claim")
                        .ok_or_else(|| CampaignError::Integrity("claim absent".into()))?,
                )?,
                establishment_id: text_field(c, "establishmentId")?,
                establishment_sha256: text_field(c, "establishmentSha256")?,
                establishment_request_sha256: text_field(c, "establishmentRequestSha256")?,
                establishment_admission_sha256: text_field(c, "establishmentAdmissionSha256")?,
                observation_id: c
                    .get("observationId")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                observation_sha256: c
                    .get("observationSha256")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            })
        }
        _ => return Err(CampaignError::Unsupported("not a CE3 stage".into())),
    };
    if command_value(&request)? != stage.command
        || production_path_request_sha256(&request)
            .map_err(|e| CampaignError::Integrity(e.to_string()))?
            != stage.request_sha256
        || binding_value(match &request {
            ProductionPathRequest::RecordObservation(c) => &c.admission,
            ProductionPathRequest::EstablishClaim(c) => &c.admission,
            ProductionPathRequest::ReadAdmission(c) => &c.admission,
        })
        .get("childRequestSha256")
        .and_then(Value::as_str)
            != Some(stage.request_sha256.as_str())
    {
        return Err(CampaignError::Integrity("CE3 stage command changed".into()));
    }
    Ok(request)
}

pub(crate) fn validate_stored_stage(
    state: &Snapshot,
    intent: &CompletionIntent,
    stage: &StageIntent,
) -> Result<()> {
    if stage.stage_id != stage_id(&intent.campaign_operation_id, stage.kind)
        || stage.registration_base_revision.as_str().is_empty()
        || stage.owner_locator.is_null()
    {
        return Err(CampaignError::Integrity(
            "saved stage identity invalid".into(),
        ));
    }
    if stage.kind == StageKind::EpisodeResultRead {
        let content = parse_episode_json(&text_field(&stage.command, "contentCanonicalJson")?)
            .map_err(|e| CampaignError::Integrity(e.to_string()))?;
        let digest = episode_value_digest(&content);
        if digest != stage.request_sha256
            || digest != stage.original_admission_sha256
            || stage.original_admission != Value::Null
            || text_field(&stage.command, "contentDigest")? != digest
            || stage.owner_locator["contentDigest"] != digest
        {
            return Err(CampaignError::Integrity(
                "episode stage content differs".into(),
            ));
        }
    } else {
        let request = stage_request(stage, ProductionPathAccess::Original)?;
        let binding = match &request {
            ProductionPathRequest::RecordObservation(c) => &c.admission,
            ProductionPathRequest::EstablishClaim(c) => &c.admission,
            ProductionPathRequest::ReadAdmission(c) => &c.admission,
        };
        let prepared = state
            .obligations
            .iter()
            .find(|o| o.obligation_id == intent.obligation_id)
            .and_then(|o| o.request.as_ref())
            .ok_or_else(|| CampaignError::Integrity("prepared request absent".into()))?;
        let dispatch = state
            .progress
            .attempts
            .iter()
            .find(|a| a.attempt_id == intent.attempt_id)
            .and_then(|a| a.dispatch.as_ref())
            .ok_or_else(|| CampaignError::Integrity("dispatch absent".into()))?;
        let expected_kind = match stage.kind {
            StageKind::Observation => ProductionPathStage::RecordObservation,
            StageKind::EstablishmentBuilder | StageKind::EstablishmentCampaign => {
                ProductionPathStage::EstablishClaim
            }
            StageKind::AdmissionBuilder | StageKind::AdmissionCampaign => {
                ProductionPathStage::ReadAdmission
            }
            _ => return Err(CampaignError::Integrity("reserved stage persisted".into())),
        };
        if binding.stage != expected_kind
            || binding.campaign_root_id != prepared.root_id
            || binding.campaign_revision != stage.registration_base_revision.as_str()
            || binding.campaign_operation_id != intent.campaign_operation_id
            || binding.obligation_id != intent.obligation_id
            || binding.candidate_digest != prepared.candidate.receipt_sha256
            || binding.selection_digest != prepared.selection.campaign_digest
            || binding.profile_digest != prepared.profile_digest
            || binding.prepared_request_sha256 != prepared.request_digest
            || binding.review_episode_id != dispatch.command.episode_id
            || binding.attempt_id != intent.attempt_id
            || binding.native_result_sha256 != intent.native_result_claim_sha256
        {
            return Err(CampaignError::Integrity(
                "saved stage binding differs".into(),
            ));
        }
        let locator = locator_from_value(&stage.owner_locator)?;
        if locator.root.root_id.is_empty()
            || locator.stage != expected_kind
            || locator.payload_sha256 != stage.request_sha256
            || locator.admission_sha256 != stage.original_admission_sha256
        {
            return Err(CampaignError::Integrity("saved CE locator differs".into()));
        }
    }
    match &stage.progress {
        StageProgress::Registered => {}
        StageProgress::OwnerCommitted { receipt }
            if matches!(
                stage.kind,
                StageKind::Observation
                    | StageKind::EstablishmentBuilder
                    | StageKind::EstablishmentCampaign
            ) =>
        {
            let receipt = receipt_from_value(receipt)?;
            let locator = locator_from_value(&stage.owner_locator)?;
            let mut expected = locator;
            expected.custody_sha256 = receipt.locator.custody_sha256.clone();
            if receipt.locator != expected {
                return Err(CampaignError::Integrity(
                    "saved CE receipt locator differs".into(),
                ));
            }
        }
        StageProgress::Checked {
            exact_refs,
            content_digests,
        } if matches!(
            stage.kind,
            StageKind::AdmissionBuilder
                | StageKind::AdmissionCampaign
                | StageKind::EpisodeResultRead
        ) =>
        {
            if exact_refs.len() != 1
                || content_digests.len() != 1
                || campaign_digest(&exact_refs[0])? != content_digests[0]
            {
                return Err(CampaignError::Integrity(
                    "saved owner readback digest differs".into(),
                ));
            }
        }
        StageProgress::Conflicting { .. } | StageProgress::Unresolved { .. } => {}
        _ => {
            return Err(CampaignError::Integrity(
                "saved stage progress kind differs".into(),
            ));
        }
    }
    Ok(())
}

impl CampaignInner {
    fn check_production_path_admission(
        &self,
        binding: &ProductionPathAdmissionBinding,
        request: &ProductionPathRequest,
    ) -> Result<ExecutionEvidenceRef> {
        binding
            .validate()
            .map_err(|e| CampaignError::Contract(e.to_string()))?;
        let join = self
            .initial_join
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("initial join not installed".into()))?;
        if binding.campaign_root_id != self.store.root_id
            || binding.campaign_operation_id.is_empty()
        {
            return Err(CampaignError::Conflict(
                "CE3 campaign root/operation differs".into(),
            ));
        }
        let state = self
            .store
            .read(&self.config.identity)?
            .ok_or_else(|| CampaignError::Integrity("campaign absent".into()))?;
        let current = state
            .obligations
            .iter()
            .find(|o| o.obligation_id == binding.obligation_id && o.status == "executing")
            .and_then(|o| o.request.as_ref())
            .ok_or_else(|| CampaignError::Conflict("CE3 active obligation absent".into()))?;
        let attempt = state
            .progress
            .attempts
            .iter()
            .find(|a| a.preparation_operation_id == current.operation_id)
            .ok_or_else(|| CampaignError::Integrity("CE3 attempt absent".into()))?;
        let dispatch = attempt
            .dispatch
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("CE3 dispatch absent".into()))?;
        if !dispatch.may_have_entered
            || self
                .store
                .dispatch_slot(&self.config.identity, &binding.obligation_id)?
                != Some((Some(dispatch.operation_id.clone()), true))
        {
            return Err(CampaignError::Conflict("CE3 dispatch slot differs".into()));
        }
        let intent = attempt
            .completion_intent
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("completion intent absent".into()))?;
        if intent.outcome.is_some()
            || attempt.outcome.is_some()
            || intent.owner_selection_digest != join.digest
            || intent.campaign_operation_id != binding.campaign_operation_id
            || intent.obligation_id != binding.obligation_id
            || intent.prepared_request_sha256 != current.request_digest
            || intent.attempt_id != attempt.attempt_id
            || binding.candidate_digest != current.candidate.receipt_sha256
            || binding.selection_digest != current.selection.campaign_digest
            || binding.profile_digest != current.profile_digest
            || binding.prepared_request_sha256 != current.request_digest
            || binding.review_episode_id != dispatch.command.episode_id
            || binding.attempt_id != attempt.attempt_id
            || binding.native_result_sha256 != intent.native_result_claim_sha256
        {
            return Err(CampaignError::Conflict(
                "CE3 current subject differs".into(),
            ));
        }
        let request_sha = production_path_request_sha256(request)
            .map_err(|e| CampaignError::Contract(e.to_string()))?;
        if binding.child_request_sha256 != request_sha {
            return Err(CampaignError::Conflict("CE3 request hash differs".into()));
        }
        let stage = intent
            .stages
            .iter()
            .find(|s| {
                s.request_sha256 == request_sha
                    && s.original_admission_sha256 == binding.original_digest()
            })
            .ok_or_else(|| CampaignError::Conflict("CE3 stage not registered".into()))?;
        let original = decode_binding(&stage.original_admission)?;
        if original.access != ProductionPathAccess::Original
            || original.original_digest() != binding.original_digest()
            || stage.owner_selection_digest != join.digest
            || stage.evidence_ref != intent.execution_evidence_ref
            || stage.command != command_value(request)?
        {
            return Err(CampaignError::Conflict("CE3 original stage differs".into()));
        }
        let registration = self
            .store
            .receipt(&stage.registration_operation_id)?
            .ok_or_else(|| CampaignError::Integrity("CE3 registration receipt absent".into()))?;
        if registration.prior_revision.as_ref() != Some(&stage.registration_base_revision)
            || registration.identity != self.config.identity
            || registration.root_id != self.store.root_id
            || registration.result_revision == stage.registration_base_revision
            || registration.kind != "register_initial_stage"
                && registration.kind != "prepare_initial_completion"
            || (binding.access == ProductionPathAccess::Original
                && state.revision != registration.result_revision)
        {
            return Err(CampaignError::Conflict(
                "CE3 registration revision differs".into(),
            ));
        }
        if stage.kind.is_j2() || stage.owner_locator.is_null() {
            return Err(CampaignError::Conflict("CE3 locator absent".into()));
        }
        intent.execution_evidence_ref.checked()
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InitialCompletionLocator {
    pub campaign_root_id: String,
    pub anchored_root: PathBuf,
    pub identity: CampaignIdentity,
    pub obligation_id: String,
    pub attempt_id: String,
    pub completion_operation_id: String,
    pub preparation_request_sha256: String,
    pub config_digest: String,
    pub original_registration: RecoveryLocator,
}
#[derive(Clone, Debug)]
pub struct InitialCompletionPrepared {
    pub snapshot: Snapshot,
    pub locator: InitialCompletionLocator,
    pub stage_ids: Vec<String>,
}
#[derive(Clone, Debug)]
pub enum InitialProgressState {
    Pending {
        stage: StageKind,
        reason: String,
    },
    OwnerReadsJoined,
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
pub struct InitialCompletionProgress {
    pub locator: InitialCompletionLocator,
    pub last_checked_revision: CampaignRevision,
    pub stages: Vec<StageIntent>,
    pub newly_observed_receipts: Vec<Value>,
    pub state: InitialProgressState,
}

fn stage_id(completion: &str, kind: StageKind) -> String {
    format!(
        "{completion}:{}",
        match kind {
            StageKind::Observation => "observation",
            StageKind::EstablishmentBuilder => "establishment-builder",
            StageKind::EstablishmentCampaign => "establishment-campaign",
            StageKind::AdmissionBuilder => "admission-builder",
            StageKind::AdmissionCampaign => "admission-campaign",
            StageKind::EpisodeResultRead => "episode-result-read",
            _ => "j2-reserved",
        }
    )
}
fn claim_selection(state: &Snapshot, obligation: &str) -> Result<Vec<(SelectedClaim, ClaimValue)>> {
    let selection = state
        .review_selection
        .as_ref()
        .ok_or_else(|| CampaignError::Integrity("review selection absent".into()))?;
    let specialist = selection["specialists"]
        .as_array()
        .and_then(|items| items.iter().find(|item| item["obligationId"] == obligation))
        .ok_or_else(|| CampaignError::Conflict("selected specialist absent".into()))?;
    if specialist["selection"] != "selected" {
        return Err(CampaignError::Conflict("specialist is not selected".into()));
    }
    let docs = specialist["requiredClaims"]
        .as_array()
        .ok_or_else(|| CampaignError::Integrity("required claims absent".into()))?;
    let mut result = Vec::new();
    for boundary in ["builder_projection", "campaign_terminalization"] {
        let document = docs
            .iter()
            .find(|v| v["consumptionBoundary"] == boundary)
            .ok_or_else(|| CampaignError::Integrity("selected boundary absent".into()))?;
        let exact = claim_from_value(document)?;
        claim_evidence::validate_production_path_claim(&exact)
            .map_err(|e| CampaignError::Contract(e.to_string()))?;
        let selected = SelectedClaim {
            claim_id: text_field(document, "claimId")?,
            revision: text_field(document, "revision")?,
            document_sha256: claim_digest(&exact)
                .map_err(|e| CampaignError::Contract(e.to_string()))?,
            boundary: boundary.into(),
            consumer: text_field(document, "consumer")?,
        };
        result.push((selected, exact));
    }
    if docs.len() != 2 || result[0].0.claim_id == result[1].0.claim_id {
        return Err(CampaignError::Integrity(
            "selected claim pair differs".into(),
        ));
    }
    Ok(result)
}
fn checked_execution_for_attempt(
    owner: &CampaignInner,
    state: &Snapshot,
    obligation_id: &str,
    reference: &ExecutionEvidenceRef,
    checked: &CheckedExecutionResult,
) -> Result<(String, Vec<SelectedClaim>, ObservationSelection)> {
    let join = owner
        .initial_join
        .as_ref()
        .ok_or_else(|| CampaignError::Conflict("initial join not installed".into()))?;
    join.selection
        .evidence_profile
        .require_class(checked.evidence_class())
        .map_err(|e| CampaignError::Unsupported(e.to_string()))?;
    if reference != checked.reference()
        || reference.owner != join.selection.evidence_owner
        || reference.root_id != join.selection.evidence_root_id
        || reference.profile != join.selection.evidence_record_profile
    {
        return Err(CampaignError::Conflict(
            "execution evidence reference differs".into(),
        ));
    }
    let request = state
        .obligations
        .iter()
        .find(|o| o.obligation_id == obligation_id && o.status == "executing")
        .and_then(|o| o.request.as_ref())
        .ok_or_else(|| CampaignError::Conflict("active request absent".into()))?;
    let attempt = state
        .progress
        .attempts
        .iter()
        .find(|a| a.preparation_operation_id == request.operation_id)
        .ok_or_else(|| CampaignError::Integrity("active attempt absent".into()))?;
    let dispatch = attempt
        .dispatch
        .as_ref()
        .ok_or_else(|| CampaignError::Conflict("dispatch absent".into()))?;
    let dispatch_receipt = owner
        .store
        .receipt(&dispatch.operation_id)?
        .ok_or_else(|| CampaignError::Integrity("dispatch receipt absent".into()))?;
    let binding = &checked.provenance().binding;
    if !dispatch.may_have_entered
        || owner.store.dispatch_slot(&state.identity, obligation_id)?
            != Some((Some(dispatch.operation_id.clone()), true))
        || reference.attempt_id != attempt.attempt_id
        || binding.campaign_root_id != owner.store.root_id
        || binding.obligation_id != obligation_id
        || binding.candidate_digest != request.candidate.receipt_sha256
        || binding.selection_digest != request.selection.campaign_digest
        || binding.profile_digest != request.profile_digest
        || binding.prepared_request_sha256 != request.request_digest
        || binding.review_episode_id != dispatch.command.episode_id
        || binding.attempt_id != attempt.attempt_id
        || checked.provenance().dispatch_operation_id != dispatch.operation_id
        || checked.provenance().dispatch_revision != dispatch_receipt.result_revision.as_str()
    {
        return Err(CampaignError::Conflict(
            "execution provenance differs from dispatch".into(),
        ));
    }
    let result_text = std::str::from_utf8(checked.claim_canonical_bytes())
        .map_err(|_| CampaignError::Contract("native result canonical bytes invalid".into()))?;
    let episode_value =
        parse_episode_json(result_text).map_err(|e| CampaignError::Contract(e.to_string()))?;
    let result =
        ReviewResult::parse(episode_value).map_err(|e| CampaignError::Contract(e.to_string()))?;
    let expected_subject = review_episode_core::codec::JsValue::object([
        ("commit", EpisodeValue::text(&request.candidate.commit)),
        ("tree", EpisodeValue::text(&request.candidate.tree)),
        (
            "patchIdentity",
            EpisodeValue::text(&request.candidate.patch_identity),
        ),
    ]);
    if result.value.get("subject") != Some(&expected_subject) {
        return Err(CampaignError::Conflict(
            "native result candidate differs".into(),
        ));
    }
    let claims = claim_selection(state, obligation_id)?;
    let observation = match checked.observation() {
        CheckedObservation::Present(value) => {
            claim_evidence::validate_production_path_observation(value)
                .map_err(|e| CampaignError::Contract(e.to_string()))?;
            let execution = value
                .get("execution")
                .ok_or_else(|| CampaignError::Integrity("execution absent".into()))?;
            let selection = value
                .get("selection")
                .ok_or_else(|| CampaignError::Integrity("selection absent".into()))?;
            let subject = value
                .get("subject")
                .ok_or_else(|| CampaignError::Integrity("subject absent".into()))?;
            if claim_text_field(execution, "resultDigest")? != checked.claim_sha256()
                || claim_text_field(execution, "attemptId")? != attempt.attempt_id
                || claim_text_field(selection, "revision")? != request.selection.campaign_digest
                || claim_text_field(subject, "reviewEpisodeId")? != dispatch.command.episode_id
                || claim_text_field(value, "obligationId")? != obligation_id
            {
                return Err(CampaignError::Conflict(
                    "observation execution subject differs".into(),
                ));
            }
            ObservationSelection::Present {
                observation_id: claim_text_field(value, "id")?,
                event_identity: claim_text_field(value, "event_identity")?,
                observation_sha256: claim_digest(value)
                    .map_err(|e| CampaignError::Contract(e.to_string()))?,
            }
        }
        CheckedObservation::Absent { reason } => ObservationSelection::Absent {
            reason: reason.clone(),
            evidence_ref: ExecutionRefRecord::from(reference),
        },
    };
    Ok((
        dispatch.operation_id.clone(),
        claims.into_iter().map(|c| c.0).collect(),
        observation,
    ))
}

impl Campaign {
    pub fn prepare_initial_completion(
        &mut self,
        identity: &CampaignIdentity,
        expected: &CampaignRevision,
        obligation_id: &str,
        operation_id: &str,
        evidence_ref: &ExecutionEvidenceRef,
    ) -> Result<Effect<InitialCompletionPrepared>> {
        require_text(operation_id, "completion operation")?;
        evidence_ref
            .validate()
            .map_err(|e| CampaignError::Contract(e.to_string()))?;
        let evidence = {
            let owner = self.owner()?;
            owner.check_identity(identity)?;
            owner.current(identity, expected)?;
            Arc::clone(
                &owner
                    .initial_join
                    .as_ref()
                    .ok_or_else(|| CampaignError::Conflict("initial join not installed".into()))?
                    .evidence,
            )
        };
        // Bounded owner read happens without the campaign mutex; the state is
        // checked again under the gate before the intent CAS.
        let checked = evidence
            .read_result(evidence_ref)
            .map_err(|e| CampaignError::Integrity(e.to_string()))?;
        let mut owner = self.owner()?;
        let state = owner.current(identity, expected)?;
        let (dispatch_operation_id, selected_claims, observation_selection) =
            checked_execution_for_attempt(&owner, &state, obligation_id, evidence_ref, &checked)?;
        let current = state
            .obligations
            .iter()
            .find(|o| o.obligation_id == obligation_id)
            .and_then(|o| o.request.as_ref())
            .ok_or_else(|| CampaignError::Conflict("prepared request absent".into()))?;
        let preparation_operation_id = current.operation_id.clone();
        let prepared_request_sha256 = current.request_digest.clone();
        let join_digest = owner.initial_join.as_ref().unwrap().digest.clone();
        let intent = CompletionIntent {
            campaign_operation_id: operation_id.into(),
            registration_operation_id: operation_id.into(),
            registration_base_revision: expected.clone(),
            obligation_id: obligation_id.into(),
            attempt_id: evidence_ref.attempt_id.clone(),
            preparation_operation_id: preparation_operation_id.clone(),
            prepared_request_sha256,
            dispatch_operation_id,
            owner_selection_digest: join_digest,
            execution_evidence_ref: ExecutionRefRecord::from(evidence_ref),
            native_result_claim_sha256: checked.claim_sha256().into(),
            native_result_raw_sha256: checked.raw_sha256().into(),
            execution_session_id: checked.provenance().execution_session_id.clone(),
            observation_selection,
            selected_claims,
            stages: Vec::new(),
            consumption_intents: Vec::new(),
            joined_readbacks: None,
            outcome: None,
        };
        let digest = campaign_digest(&json!({"identity":identity,"expectedRevision":expected,
            "obligationId":obligation_id,"operationId":operation_id,"intent":intent}))?;
        if let Some((receipt, replayed)) = owner.replay(
            operation_id,
            "prepare_initial_completion",
            &digest,
            identity,
        )? {
            let locator = completion_locator(
                &owner,
                &replayed,
                obligation_id,
                operation_id,
                receipt.request_digest.clone(),
                expected.clone(),
            )?;
            return Ok(Effect::Replayed {
                value: prepared(replayed, locator),
                receipt,
            });
        }
        let mut next = state;
        let attempt = next
            .progress
            .attempts
            .iter_mut()
            .find(|a| a.preparation_operation_id == preparation_operation_id)
            .ok_or_else(|| CampaignError::Integrity("active attempt absent".into()))?;
        if attempt.completion_intent.is_some() || attempt.outcome.is_some() {
            return Ok(Effect::NoEffect("completion intent already bound".into()));
        }
        let dispatch = attempt
            .dispatch
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("dispatch absent".into()))?;
        intent.validate_initial(dispatch, &attempt.attempt_id)?;
        attempt.completion_intent = Some(intent);
        next.revision = state_revision(&next)?;
        preflight_snapshot(&next)?;
        let locator = completion_locator(
            &owner,
            &next,
            obligation_id,
            operation_id,
            digest.clone(),
            expected.clone(),
        )?;
        Ok(
            match owner.put(
                next,
                Some(expected.clone()),
                operation_id,
                "prepare_initial_completion",
                digest,
                false,
                None,
            ) {
                Effect::Applied { value, receipt } => {
                    crate::fault_cut("j1_after_prepare_commit");
                    Effect::Applied {
                        value: prepared(value, locator),
                        receipt,
                    }
                }
                Effect::Replayed { value, receipt } => Effect::Replayed {
                    value: prepared(value, locator),
                    receipt,
                },
                Effect::NoEffect(reason) => Effect::NoEffect(reason),
                Effect::OutcomeUnknown(locator) => Effect::OutcomeUnknown(locator),
            },
        )
    }
}
fn preflight_snapshot(state: &Snapshot) -> Result<()> {
    if serde_json::to_vec(state)
        .map_err(|e| CampaignError::Contract(e.to_string()))?
        .len()
        > 1_048_576
    {
        return Err(CampaignError::Contract(
            "encoded campaign exceeds capacity".into(),
        ));
    }
    Ok(())
}
fn completion_locator(
    owner: &CampaignInner,
    state: &Snapshot,
    obligation_id: &str,
    operation_id: &str,
    digest: String,
    base: CampaignRevision,
) -> Result<InitialCompletionLocator> {
    let request = state
        .obligations
        .iter()
        .find(|o| o.obligation_id == obligation_id)
        .and_then(|o| o.request.as_ref())
        .ok_or_else(|| CampaignError::Integrity("request absent".into()))?;
    let attempt = state
        .progress
        .attempts
        .iter()
        .find(|a| a.preparation_operation_id == request.operation_id)
        .ok_or_else(|| CampaignError::Integrity("attempt absent".into()))?;
    Ok(InitialCompletionLocator {
        campaign_root_id: owner.store.root_id.clone(),
        anchored_root: owner.store.root.clone(),
        identity: state.identity.clone(),
        obligation_id: obligation_id.into(),
        attempt_id: attempt.attempt_id.clone(),
        completion_operation_id: operation_id.into(),
        preparation_request_sha256: request.request_digest.clone(),
        config_digest: owner.config.digest.clone(),
        original_registration: owner.locator(
            &state.identity,
            operation_id,
            "prepare_initial_completion",
            digest,
            Some(base),
        ),
    })
}
fn prepared(state: Snapshot, locator: InitialCompletionLocator) -> InitialCompletionPrepared {
    let op = &locator.completion_operation_id;
    let mut stage_ids = vec![
        stage_id(op, StageKind::EstablishmentBuilder),
        stage_id(op, StageKind::EstablishmentCampaign),
        stage_id(op, StageKind::AdmissionBuilder),
        stage_id(op, StageKind::AdmissionCampaign),
        stage_id(op, StageKind::EpisodeResultRead),
    ];
    if state
        .progress
        .attempts
        .iter()
        .find(|a| a.attempt_id == locator.attempt_id)
        .and_then(|a| a.completion_intent.as_ref())
        .is_some_and(|i| {
            matches!(
                i.observation_selection,
                ObservationSelection::Present { .. }
            )
        })
    {
        stage_ids.insert(0, stage_id(op, StageKind::Observation));
    }
    InitialCompletionPrepared {
        snapshot: state,
        locator,
        stage_ids,
    }
}

fn locator_value(v: &ProductionPathLocator) -> Value {
    json!({"root":{"absolutePath":v.root.absolute_path,"rootId":v.root.root_id,
        "profile":v.root.profile},"stage":match v.stage {
            ProductionPathStage::RecordObservation=>"record_observation",
            ProductionPathStage::EstablishClaim=>"establish_claim",
            ProductionPathStage::ReadAdmission=>"read_admission"},
        "key":v.key,"payloadSha256":v.payload_sha256,"grantSha256":v.grant_sha256,
        "admissionSha256":v.admission_sha256,"custodySha256":v.custody_sha256})
}
fn locator_from_value(v: &Value) -> Result<ProductionPathLocator> {
    let root = &v["root"];
    let stage = match text_field(v, "stage")?.as_str() {
        "record_observation" => ProductionPathStage::RecordObservation,
        "establish_claim" => ProductionPathStage::EstablishClaim,
        "read_admission" => ProductionPathStage::ReadAdmission,
        _ => return Err(CampaignError::Integrity("CE locator stage invalid".into())),
    };
    let locator = ProductionPathLocator {
        root: RootIdentity {
            absolute_path: PathBuf::from(text_field(root, "absolutePath")?),
            root_id: text_field(root, "rootId")?,
            profile: text_field(root, "profile")?,
        },
        stage,
        key: text_field(v, "key")?,
        payload_sha256: text_field(v, "payloadSha256")?,
        grant_sha256: text_field(v, "grantSha256")?,
        admission_sha256: text_field(v, "admissionSha256")?,
        custody_sha256: v
            .get("custodySha256")
            .and_then(Value::as_str)
            .map(str::to_owned),
    };
    for sha in [
        &locator.payload_sha256,
        &locator.grant_sha256,
        &locator.admission_sha256,
    ] {
        crate::require_sha(sha, "CE locator digest")?;
    }
    Ok(locator)
}
fn receipt_value(v: &claim_evidence::ProductionPathReceipt) -> Value {
    json!({"id":v.id,"sha256":v.sha256,"locator":locator_value(&v.locator)})
}
fn receipt_from_value(v: &Value) -> Result<claim_evidence::ProductionPathReceipt> {
    let receipt = claim_evidence::ProductionPathReceipt {
        id: text_field(v, "id")?,
        sha256: text_field(v, "sha256")?,
        locator: locator_from_value(
            v.get("locator")
                .ok_or_else(|| CampaignError::Integrity("CE receipt locator absent".into()))?,
        )?,
    };
    crate::require_sha(&receipt.sha256, "CE receipt")?;
    Ok(receipt)
}
fn write_receipt(publication: ProductionPathPublication) -> claim_evidence::ProductionPathReceipt {
    match publication {
        ProductionPathPublication::Applied(r) | ProductionPathPublication::Replayed(r) => r,
    }
}
fn current_intent<'a>(
    state: &'a Snapshot,
    obligation: &str,
    operation: &str,
) -> Result<&'a CompletionIntent> {
    let request = state
        .obligations
        .iter()
        .find(|o| o.obligation_id == obligation && o.status == "executing")
        .and_then(|o| o.request.as_ref())
        .ok_or_else(|| CampaignError::Conflict("active request absent".into()))?;
    let attempt = state
        .progress
        .attempts
        .iter()
        .find(|a| a.preparation_operation_id == request.operation_id)
        .ok_or_else(|| CampaignError::Integrity("active attempt absent".into()))?;
    let intent = attempt
        .completion_intent
        .as_ref()
        .ok_or_else(|| CampaignError::Conflict("completion intent absent".into()))?;
    if intent.campaign_operation_id != operation
        || intent.prepared_request_sha256 != request.request_digest
        || intent.attempt_id != attempt.attempt_id
        || attempt.outcome.is_some()
        || intent.outcome.is_some()
    {
        return Err(CampaignError::Conflict(
            "completion no longer current".into(),
        ));
    }
    Ok(intent)
}
fn current_intent_mut<'a>(
    state: &'a mut Snapshot,
    obligation: &str,
    operation: &str,
) -> Result<&'a mut CompletionIntent> {
    let request = state
        .obligations
        .iter()
        .find(|o| o.obligation_id == obligation && o.status == "executing")
        .and_then(|o| o.request.as_ref())
        .ok_or_else(|| CampaignError::Conflict("active request absent".into()))?;
    let attempt = state
        .progress
        .attempts
        .iter_mut()
        .find(|a| a.preparation_operation_id == request.operation_id)
        .ok_or_else(|| CampaignError::Integrity("active attempt absent".into()))?;
    let intent = attempt
        .completion_intent
        .as_mut()
        .ok_or_else(|| CampaignError::Conflict("completion intent absent".into()))?;
    if intent.campaign_operation_id != operation
        || intent.prepared_request_sha256 != request.request_digest
        || intent.attempt_id != attempt.attempt_id
        || attempt.outcome.is_some()
        || intent.outcome.is_some()
    {
        return Err(CampaignError::Conflict(
            "completion no longer current".into(),
        ));
    }
    Ok(intent)
}
fn stage_by_kind(intent: &CompletionIntent, kind: StageKind) -> Option<&StageIntent> {
    intent.stages.iter().find(|s| s.kind == kind)
}
fn stage_by_kind_mut(intent: &mut CompletionIntent, kind: StageKind) -> Option<&mut StageIntent> {
    intent.stages.iter_mut().find(|s| s.kind == kind)
}
fn committed_receipt(
    intent: &CompletionIntent,
    kind: StageKind,
) -> Result<Option<claim_evidence::ProductionPathReceipt>> {
    match stage_by_kind(intent, kind).map(|s| &s.progress) {
        Some(StageProgress::OwnerCommitted { receipt }) => Ok(Some(receipt_from_value(receipt)?)),
        Some(StageProgress::Checked { .. }) => Err(CampaignError::Integrity(
            "write stage has read progress".into(),
        )),
        _ => Ok(None),
    }
}
fn stage_binding(
    intent: &CompletionIntent,
    state: &Snapshot,
    kind: StageKind,
    operation_id: &str,
) -> Result<ProductionPathAdmissionBinding> {
    let request = state
        .obligations
        .iter()
        .find(|o| o.obligation_id == intent.obligation_id)
        .and_then(|o| o.request.as_ref())
        .ok_or_else(|| CampaignError::Integrity("request absent".into()))?;
    let dispatch = state
        .progress
        .attempts
        .iter()
        .find(|a| a.attempt_id == intent.attempt_id)
        .and_then(|a| a.dispatch.as_ref())
        .ok_or_else(|| CampaignError::Integrity("dispatch absent".into()))?;
    let stage = match kind {
        StageKind::Observation => ProductionPathStage::RecordObservation,
        StageKind::EstablishmentBuilder | StageKind::EstablishmentCampaign => {
            ProductionPathStage::EstablishClaim
        }
        StageKind::AdmissionBuilder | StageKind::AdmissionCampaign => {
            ProductionPathStage::ReadAdmission
        }
        _ => return Err(CampaignError::Contract("CE stage kind invalid".into())),
    };
    let _ = operation_id;
    Ok(ProductionPathAdmissionBinding {
        campaign_root_id: request.root_id.clone(),
        campaign_revision: state.revision.as_str().into(),
        campaign_operation_id: intent.campaign_operation_id.clone(),
        obligation_id: intent.obligation_id.clone(),
        candidate_digest: request.candidate.receipt_sha256.clone(),
        selection_digest: request.selection.campaign_digest.clone(),
        profile_digest: request.profile_digest.clone(),
        prepared_request_sha256: request.request_digest.clone(),
        child_request_sha256: String::new(),
        review_episode_id: dispatch.command.episode_id.clone(),
        attempt_id: intent.attempt_id.clone(),
        native_result_sha256: intent.native_result_claim_sha256.clone(),
        episode_revision: None,
        session_id: match intent.observation_selection {
            ObservationSelection::Present { .. } => intent.execution_session_id.clone(),
            ObservationSelection::Absent { .. } => None,
        },
        stage,
        access: ProductionPathAccess::Original,
    })
}
fn set_request_hash(request: &mut ProductionPathRequest) -> Result<()> {
    let hash = production_path_request_sha256(request)
        .map_err(|e| CampaignError::Contract(e.to_string()))?;
    match request {
        ProductionPathRequest::RecordObservation(c) => c.admission.child_request_sha256 = hash,
        ProductionPathRequest::EstablishClaim(c) => c.admission.child_request_sha256 = hash,
        ProductionPathRequest::ReadAdmission(c) => c.admission.child_request_sha256 = hash,
    }
    Ok(())
}
fn stage_request_new(
    intent: &CompletionIntent,
    state: &Snapshot,
    kind: StageKind,
    checked: &CheckedExecutionResult,
    grant: &str,
) -> Result<ProductionPathRequest> {
    let id = stage_id(&intent.campaign_operation_id, kind);
    let binding = stage_binding(intent, state, kind, &id)?;
    let mut request = match kind {
        StageKind::Observation => {
            let CheckedObservation::Present(obs) = checked.observation() else {
                return Err(CampaignError::Conflict(
                    "observation absent in evidence".into(),
                ));
            };
            ProductionPathRequest::RecordObservation(ObservationCommand {
                grant_id: grant.into(),
                admission: binding,
                observation: obs.clone(),
            })
        }
        StageKind::EstablishmentBuilder | StageKind::EstablishmentCampaign => {
            let boundary = if kind == StageKind::EstablishmentBuilder {
                "builder_projection"
            } else {
                "campaign_terminalization"
            };
            let claim = claim_selection(state, &intent.obligation_id)?
                .into_iter()
                .find(|(s, _)| s.boundary == boundary)
                .ok_or_else(|| CampaignError::Integrity("claim absent".into()))?
                .1;
            let observation_id = match &intent.observation_selection {
                ObservationSelection::Present { observation_id, .. } => {
                    Some(observation_id.clone())
                }
                ObservationSelection::Absent { .. } => None,
            };
            ProductionPathRequest::EstablishClaim(EstablishmentCommand {
                grant_id: grant.into(),
                admission: binding,
                operation_id: id,
                claim,
                observation_id,
            })
        }
        StageKind::AdmissionBuilder | StageKind::AdmissionCampaign => {
            let establishment_kind = if kind == StageKind::AdmissionBuilder {
                StageKind::EstablishmentBuilder
            } else {
                StageKind::EstablishmentCampaign
            };
            let receipt = committed_receipt(intent, establishment_kind)?
                .ok_or_else(|| CampaignError::Conflict("establishment receipt pending".into()))?;
            let boundary = if kind == StageKind::AdmissionBuilder {
                "builder_projection"
            } else {
                "campaign_terminalization"
            };
            let claim = claim_selection(state, &intent.obligation_id)?
                .into_iter()
                .find(|(s, _)| s.boundary == boundary)
                .ok_or_else(|| CampaignError::Integrity("claim absent".into()))?
                .1;
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
                grant_id: grant.into(),
                admission: binding,
                claim,
                establishment_id: receipt.id,
                establishment_sha256: receipt.sha256,
                establishment_request_sha256: receipt.locator.payload_sha256,
                establishment_admission_sha256: receipt.locator.admission_sha256,
                observation_id,
                observation_sha256,
            })
        }
        _ => {
            return Err(CampaignError::Unsupported(
                "stage outside J1 CE pair".into(),
            ));
        }
    };
    set_request_hash(&mut request)?;
    Ok(request)
}

fn checked_owner_wrapper(campaign: &Campaign, owners: &InitialCompletionOwners<'_>) -> Result<()> {
    let guard = campaign.owner()?;
    let join = guard
        .initial_join
        .as_ref()
        .ok_or_else(|| CampaignError::Conflict("initial join not installed".into()))?;
    if !Arc::ptr_eq(&campaign.inner, &owners.claims.campaign)
        || guard.store.epoch != owners.claims.campaign_epoch
        || join.digest != owners.claims.selection_digest
        || owners.claims.claims.root() != &join.selection.claims_root
    {
        return Err(CampaignError::Conflict(
            "initial claims owner differs".into(),
        ));
    }
    if let Some(episode) = owners.episode.as_ref()
        && episode.root().to_string_lossy() != join.selection.episode_selection.root
    {
        return Err(CampaignError::Conflict(
            "episode reader root differs".into(),
        ));
    }
    Ok(())
}
fn stage_locator_for_request(
    claims: &ClaimsApplication<CE3CampaignClaimsAdmission>,
    request: &ProductionPathRequest,
    selection: &InitialJoinSelection,
) -> Result<ProductionPathLocator> {
    let locator = match request {
        ProductionPathRequest::RecordObservation(c) => claims
            .observation_locator(c)
            .map_err(|e| CampaignError::Contract(e.to_string()))?,
        ProductionPathRequest::EstablishClaim(c) => claims
            .establishment_locator(c)
            .map_err(|e| CampaignError::Contract(e.to_string()))?,
        ProductionPathRequest::ReadAdmission(c) => ProductionPathLocator {
            root: claims.root().clone(),
            stage: ProductionPathStage::ReadAdmission,
            key: c.establishment_id.clone(),
            payload_sha256: production_path_request_sha256(request)
                .map_err(|e| CampaignError::Contract(e.to_string()))?,
            grant_sha256: selection.production_grant_sha256.clone(),
            admission_sha256: c.admission.original_digest(),
            custody_sha256: None,
        },
    };
    if locator.root != selection.claims_root
        || locator.grant_sha256 != selection.production_grant_sha256
    {
        return Err(CampaignError::Conflict(
            "selected CE grant/root differs".into(),
        ));
    }
    Ok(locator)
}
fn register_ce_stage(
    campaign: &mut Campaign,
    locator: &InitialCompletionLocator,
    kind: StageKind,
    checked: &CheckedExecutionResult,
    owners: &mut InitialCompletionOwners<'_>,
) -> Result<Effect<Snapshot>> {
    checked_owner_wrapper(campaign, owners)?;
    let mut gate = campaign.owner()?;
    let state = gate
        .store
        .read(&locator.identity)?
        .ok_or_else(|| CampaignError::Integrity("campaign absent".into()))?;
    let intent = current_intent(
        &state,
        &locator.obligation_id,
        &locator.completion_operation_id,
    )?;
    if stage_by_kind(intent, kind).is_some() {
        return Ok(Effect::NoEffect("stage already registered".into()));
    }
    let selection = gate.initial_join.as_ref().unwrap().selection.clone();
    let request = stage_request_new(
        intent,
        &state,
        kind,
        checked,
        &selection.production_grant_id,
    )?;
    let owner_locator = stage_locator_for_request(&owners.claims.claims, &request, &selection)?;
    let binding = match &request {
        ProductionPathRequest::RecordObservation(c) => &c.admission,
        ProductionPathRequest::EstablishClaim(c) => &c.admission,
        ProductionPathRequest::ReadAdmission(c) => &c.admission,
    };
    let stage_id = stage_id(&locator.completion_operation_id, kind);
    let registration_operation_id = format!("{stage_id}:register");
    let stage = StageIntent {
        stage_id,
        kind,
        registration_operation_id: registration_operation_id.clone(),
        registration_base_revision: state.revision.clone(),
        command: command_value(&request)?,
        request_sha256: production_path_request_sha256(&request)
            .map_err(|e| CampaignError::Contract(e.to_string()))?,
        original_admission: binding_value(binding),
        original_admission_sha256: binding.original_digest(),
        owner_selection_digest: gate.initial_join.as_ref().unwrap().digest.clone(),
        owner_locator: locator_value(&owner_locator),
        evidence_ref: intent.execution_evidence_ref.clone(),
        progress: StageProgress::Registered,
    };
    let digest = campaign_digest(&json!({"identity":locator.identity,
        "expectedRevision":state.revision,"completionOperationId":locator.completion_operation_id,
        "registrationOperationId":registration_operation_id,"stage":stage}))?;
    if let Some((receipt, replayed)) = gate.replay(
        &registration_operation_id,
        "register_initial_stage",
        &digest,
        &locator.identity,
    )? {
        return Ok(Effect::Replayed {
            value: replayed,
            receipt,
        });
    }
    let mut next = state.clone();
    current_intent_mut(
        &mut next,
        &locator.obligation_id,
        &locator.completion_operation_id,
    )?
    .stages
    .push(stage);
    next.revision = state_revision(&next)?;
    preflight_snapshot(&next)?;
    Ok(gate.put(
        next,
        Some(state.revision),
        &registration_operation_id,
        "register_initial_stage",
        digest,
        false,
        None,
    ))
}
fn update_stage_progress(
    campaign: &mut Campaign,
    locator: &InitialCompletionLocator,
    kind: StageKind,
    new_progress: StageProgress,
) -> Result<Effect<Snapshot>> {
    let mut gate = campaign.owner()?;
    let state = gate
        .store
        .read(&locator.identity)?
        .ok_or_else(|| CampaignError::Integrity("campaign absent".into()))?;
    let stage = current_intent(
        &state,
        &locator.obligation_id,
        &locator.completion_operation_id,
    )?
    .stages
    .iter()
    .find(|s| s.kind == kind)
    .ok_or_else(|| CampaignError::Conflict("stage absent".into()))?;
    if matches!(
        (&stage.progress, &new_progress),
        (
            StageProgress::OwnerCommitted { .. },
            StageProgress::OwnerCommitted { .. }
        ) | (StageProgress::Checked { .. }, StageProgress::Checked { .. })
    ) {
        if serde_json::to_value(&stage.progress).ok() != serde_json::to_value(&new_progress).ok() {
            return Err(CampaignError::Conflict("stage progress differs".into()));
        }
        return Ok(Effect::NoEffect("stage progress already recorded".into()));
    }
    let operation_id = format!("{}:progress", stage.stage_id);
    let digest = campaign_digest(&json!({"identity":locator.identity,
        "operationId":operation_id,"stageId":stage.stage_id,
        "progress":new_progress}))?;
    if let Some((receipt, replayed)) = gate.replay(
        &operation_id,
        "record_initial_stage",
        &digest,
        &locator.identity,
    )? {
        return Ok(Effect::Replayed {
            value: replayed,
            receipt,
        });
    }
    let mut next = state.clone();
    stage_by_kind_mut(
        current_intent_mut(
            &mut next,
            &locator.obligation_id,
            &locator.completion_operation_id,
        )?,
        kind,
    )
    .ok_or_else(|| CampaignError::Integrity("stage absent".into()))?
    .progress = new_progress;
    next.revision = state_revision(&next)?;
    preflight_snapshot(&next)?;
    Ok(gate.put(
        next,
        Some(state.revision),
        &operation_id,
        "record_initial_stage",
        digest,
        false,
        None,
    ))
}
fn current_progress(
    campaign: &Campaign,
    locator: &InitialCompletionLocator,
    state: InitialProgressState,
    newly: Vec<Value>,
) -> Result<InitialCompletionProgress> {
    let snapshot = campaign
        .read(&locator.identity)?
        .ok_or_else(|| CampaignError::Integrity("campaign absent".into()))?;
    let intent = current_intent(
        &snapshot,
        &locator.obligation_id,
        &locator.completion_operation_id,
    )?;
    Ok(InitialCompletionProgress {
        locator: locator.clone(),
        last_checked_revision: snapshot.revision.clone(),
        stages: intent.stages.clone(),
        newly_observed_receipts: newly,
        state,
    })
}
fn pending(
    campaign: &Campaign,
    locator: &InitialCompletionLocator,
    kind: StageKind,
    reason: &str,
    newly: Vec<Value>,
) -> Result<InitialCompletionProgress> {
    current_progress(
        campaign,
        locator,
        InitialProgressState::Pending {
            stage: kind,
            reason: reason.into(),
        },
        newly,
    )
}

fn checked_claim_record(
    checked: &CheckedClaimEvidence,
    selected: &SelectedClaim,
    receipt: &claim_evidence::ProductionPathReceipt,
    intent: &CompletionIntent,
    selection: &InitialJoinSelection,
) -> Result<Value> {
    if checked.root() != &selection.claims_root
        || checked.binding().native_result_sha256 != intent.native_result_claim_sha256
        || checked.binding().attempt_id != intent.attempt_id
        || checked.binding().campaign_operation_id != intent.campaign_operation_id
        || checked.binding().obligation_id != intent.obligation_id
        || checked
            .boundary()
            .map_err(|e| CampaignError::Integrity(e.to_string()))?
            != selected.boundary
        || checked
            .consumer()
            .map_err(|e| CampaignError::Integrity(e.to_string()))?
            != selected.consumer
        || claim_digest(checked.claim()).map_err(|e| CampaignError::Integrity(e.to_string()))?
            != selected.document_sha256
    {
        return Err(CampaignError::Conflict(
            "CE claim readback differs from selection".into(),
        ));
    }
    let claim_ref = checked
        .claim_reference()
        .map_err(|e| CampaignError::Integrity(e.to_string()))?;
    let establishment_ref = checked
        .establishment_reference()
        .map_err(|e| CampaignError::Integrity(e.to_string()))?;
    let observation_ref = checked
        .observation_reference()
        .map_err(|e| CampaignError::Integrity(e.to_string()))?;
    if claim_ref.reference != selected.claim_id
        || claim_ref.revision != selected.revision
        || establishment_ref.reference != receipt.id
        || establishment_ref.sha256 != receipt.sha256
    {
        return Err(CampaignError::Conflict(
            "CE readback reference differs".into(),
        ));
    }
    match (&intent.observation_selection, &observation_ref) {
        (
            ObservationSelection::Present {
                observation_id,
                observation_sha256,
                ..
            },
            Some(reference),
        ) if reference.reference == *observation_id && reference.sha256 == *observation_sha256 => {}
        (ObservationSelection::Absent { .. }, None) => {}
        _ => {
            return Err(CampaignError::Conflict(
                "CE observation readback differs".into(),
            ));
        }
    }
    let custody = checked.custody();
    let evidence_ref = intent.execution_evidence_ref.checked()?;
    if custody.owner != evidence_ref.owner
        || custody.reference != evidence_ref.custody_reference()
        || custody.revision != evidence_ref.revision
        || custody.sha256 != evidence_ref.sha256
        || custody.attempt_id != intent.attempt_id
        || custody.native_result_sha256 != intent.native_result_claim_sha256
    {
        return Err(CampaignError::Conflict(
            "CE custody differs from evidence record".into(),
        ));
    }
    let observation = observation_ref
        .as_ref()
        .map(|r| value_from_claim(&r.to_value()))
        .transpose()?;
    Ok(json!({
        "claimRevisionRef":value_from_claim(&claim_ref.to_value())?,
        "establishmentRef":value_from_claim(&establishment_ref.to_value())?,
        "observationRef":observation,
        "status":checked.status().map_err(|e|CampaignError::Integrity(e.to_string()))?,
        "reasons":checked.reasons().map_err(|e|CampaignError::Integrity(e.to_string()))?,
        "boundary":selected.boundary,"consumer":selected.consumer,
        "claimDocumentSha256":selected.document_sha256,
        "custody":{"owner":custody.owner,"reference":custody.reference,
            "revision":custody.revision,"sha256":custody.sha256,
            "verifier":custody.verifier,"profile":custody.profile,
            "sessionId":custody.session_id,"observationSha256":custody.observation_sha256,
            "artifactReferences":custody.artifact_references},
        "admissionSha256":checked.binding().original_digest(),
    }))
}
fn checked_record_from_stage(stage: &StageIntent) -> Result<Value> {
    match &stage.progress {
        StageProgress::Checked {
            exact_refs,
            content_digests,
        } if exact_refs.len() == 1 && !content_digests.is_empty() => Ok(exact_refs[0].clone()),
        _ => Err(CampaignError::Conflict(
            "checked admission stage absent".into(),
        )),
    }
}
fn consumption_commitment(
    record: &Value,
    registration_operation_id: &str,
) -> Result<ConsumptionIntent> {
    let claim_ref = &record["claimRevisionRef"];
    let establishment = &record["establishmentRef"];
    let claim_revision = text_field(claim_ref, "revision")?;
    let boundary = text_field(record, "boundary")?;
    let consumer = text_field(record, "consumer")?;
    let claim_id = text_field(claim_ref, "reference")?;
    let body = json!({"schemaVersion":1,"claimRevision":claim_revision,
        "establishment":establishment,"boundary":boundary,"consumer":consumer});
    let body_claim = claim_from_value(&body)?;
    let body_digest =
        claim_digest(&body_claim).map_err(|e| CampaignError::Contract(e.to_string()))?;
    Ok(ConsumptionIntent {
        claim_revision,
        establishment: establishment.clone(),
        boundary,
        consumer,
        body_digest,
        reference: format!("claim-consumption:{claim_id}"),
        registration_operation_id: registration_operation_id.into(),
        consumed_at_operation_id: None,
    })
}
fn consumption_reference(v: &ConsumptionIntent) -> Value {
    json!({"owner":"slice-campaign","reference":v.reference,
        "revision":v.claim_revision,"sha256":v.body_digest,"freshness":"exact immutable revision"})
}
fn episode_admission(record: &Value, consumption: &ConsumptionIntent) -> Value {
    json!({"claimRevisionRef":record["claimRevisionRef"],
        "establishmentRef":record["establishmentRef"],
        "observationRef":record["observationRef"],
        "consumptionRef":consumption_reference(consumption),
        "status":record["status"],"boundary":record["boundary"],
        "consumer":record["consumer"]})
}

fn current_stage(
    campaign: &Campaign,
    locator: &InitialCompletionLocator,
    kind: StageKind,
) -> Result<(Snapshot, StageIntent, CompletionIntent)> {
    let state = campaign
        .read(&locator.identity)?
        .ok_or_else(|| CampaignError::Integrity("campaign absent".into()))?;
    let intent = current_intent(
        &state,
        &locator.obligation_id,
        &locator.completion_operation_id,
    )?
    .clone();
    let stage = stage_by_kind(&intent, kind)
        .ok_or_else(|| CampaignError::Conflict("stage absent".into()))?
        .clone();
    Ok((state, stage, intent))
}
fn stage_error_progress(
    campaign: &Campaign,
    locator: &InitialCompletionLocator,
    kind: StageKind,
    reason: String,
    newly: Vec<Value>,
) -> Result<InitialCompletionProgress> {
    current_progress(
        campaign,
        locator,
        InitialProgressState::Conflicting {
            stage: kind,
            reason,
        },
        newly,
    )
}
fn stage_unresolved_progress(
    campaign: &Campaign,
    locator: &InitialCompletionLocator,
    kind: StageKind,
    owner_locator: Value,
    reason: String,
    newly: Vec<Value>,
) -> Result<InitialCompletionProgress> {
    current_progress(
        campaign,
        locator,
        InitialProgressState::Unresolved {
            stage: kind,
            owner_locator,
            reason,
        },
        newly,
    )
}
fn write_stage(
    campaign: &mut Campaign,
    locator: &InitialCompletionLocator,
    kind: StageKind,
    owners: &mut InitialCompletionOwners<'_>,
    newly: &mut Vec<Value>,
) -> Result<Option<InitialCompletionProgress>> {
    let (state, stage, _intent) = current_stage(campaign, locator, kind)?;
    let registration = campaign
        .owner()?
        .store
        .receipt(&stage.registration_operation_id)?
        .ok_or_else(|| CampaignError::Integrity("stage registration receipt absent".into()))?;
    let access = if state.revision == registration.result_revision {
        ProductionPathAccess::Original
    } else {
        ProductionPathAccess::RecoverExact
    };
    let request = stage_request(&stage, access)?;
    let saved_locator = locator_from_value(&stage.owner_locator)?;
    let observed =
        match kind {
            StageKind::Observation => owners.claims.claims.reconcile_observation(&saved_locator),
            StageKind::EstablishmentBuilder | StageKind::EstablishmentCampaign => {
                owners.claims.claims.reconcile_establishment(
                    &saved_locator,
                    &claim_from_value(stage.command.get("claim").ok_or_else(|| {
                        CampaignError::Integrity("registered claim absent".into())
                    })?)?,
                )
            }
            _ => return Err(CampaignError::Contract("not a write stage".into())),
        };
    let receipt = match observed {
        ProductionPathReconciliation::Committed(receipt) => *receipt,
        ProductionPathReconciliation::Conflicting => {
            return Ok(Some(stage_error_progress(
                campaign,
                locator,
                kind,
                "CE exact locator conflicts".into(),
                newly.clone(),
            )?));
        }
        ProductionPathReconciliation::Unresolved(reason) => {
            return Ok(Some(stage_unresolved_progress(
                campaign,
                locator,
                kind,
                stage.owner_locator.clone(),
                reason,
                newly.clone(),
            )?));
        }
        ProductionPathReconciliation::Absent => {
            if let StageProgress::OwnerCommitted { receipt } = &stage.progress {
                return Ok(Some(stage_error_progress(
                    campaign,
                    locator,
                    kind,
                    format!(
                        "campaign saved CE receipt but owner says absent: {}",
                        receipt
                    ),
                    newly.clone(),
                )?));
            }
            let result = match request {
                ProductionPathRequest::RecordObservation(command) => {
                    owners.claims.claims.record_observation(command)
                }
                ProductionPathRequest::EstablishClaim(command) => {
                    owners.claims.claims.establish_claim(command)
                }
                _ => return Err(CampaignError::Contract("write request kind differs".into())),
            };
            match result {
                Ok(publication) => {
                    let receipt = write_receipt(publication);
                    crate::fault_cut(match kind {
                        StageKind::Observation => "j1_after_observation_commit",
                        StageKind::EstablishmentBuilder => "j1_after_establishment_builder_commit",
                        StageKind::EstablishmentCampaign => {
                            "j1_after_establishment_campaign_commit"
                        }
                        _ => "j1_unexpected_write_stage",
                    });
                    receipt
                }
                Err(ProductionPathWriteError::NoEffect(error)) => {
                    return Ok(Some(stage_error_progress(
                        campaign,
                        locator,
                        kind,
                        error.to_string(),
                        newly.clone(),
                    )?));
                }
                Err(ProductionPathWriteError::OutcomeUnknown(owner_locator)) => {
                    return Ok(Some(stage_unresolved_progress(
                        campaign,
                        locator,
                        kind,
                        locator_value(&owner_locator),
                        "CE write acknowledgement uncertain".into(),
                        newly.clone(),
                    )?));
                }
            }
        }
    };
    let mut expected_locator = saved_locator.clone();
    expected_locator.custody_sha256 = receipt.locator.custody_sha256.clone();
    if receipt.locator != expected_locator {
        return Ok(Some(stage_error_progress(
            campaign,
            locator,
            kind,
            "CE receipt differs from registered locator".into(),
            newly.clone(),
        )?));
    }
    if let StageProgress::OwnerCommitted { receipt: saved } = &stage.progress {
        let mut expected = receipt_from_value(saved)?;
        // CE reconciliation returns the exact durable establishment/operation
        // locator but omits the publication-only custody digest. The saved
        // digest remains checked by CE readback before the join.
        expected.locator.custody_sha256 = receipt.locator.custody_sha256.clone();
        if expected != receipt {
            return Ok(Some(stage_error_progress(
                campaign,
                locator,
                kind,
                "saved CE receipt differs from reconciled owner".into(),
                newly.clone(),
            )?));
        }
        newly.push(saved.clone());
        return Ok(None);
    }
    let value = receipt_value(&receipt);
    newly.push(value.clone());
    match update_stage_progress(
        campaign,
        locator,
        kind,
        StageProgress::OwnerCommitted { receipt: value },
    )? {
        Effect::Applied { .. } | Effect::Replayed { .. } | Effect::NoEffect(_) => Ok(None),
        Effect::OutcomeUnknown(campaign_locator) => Ok(Some(stage_unresolved_progress(
            campaign,
            locator,
            kind,
            serde_json::to_value(campaign_locator)
                .map_err(|e| CampaignError::Contract(e.to_string()))?,
            "campaign progress commit uncertain after CE commit".into(),
            newly.clone(),
        )?)),
    }
}
fn read_stage(
    campaign: &mut Campaign,
    locator: &InitialCompletionLocator,
    kind: StageKind,
    owners: &mut InitialCompletionOwners<'_>,
    newly: &[Value],
) -> Result<Option<InitialCompletionProgress>> {
    let (state, stage, intent) = current_stage(campaign, locator, kind)?;
    let registration = campaign
        .owner()?
        .store
        .receipt(&stage.registration_operation_id)?
        .ok_or_else(|| CampaignError::Integrity("read stage registration receipt absent".into()))?;
    let access = if state.revision == registration.result_revision {
        ProductionPathAccess::Original
    } else {
        ProductionPathAccess::RecoverExact
    };
    let request = stage_request(&stage, access)?;
    let ProductionPathRequest::ReadAdmission(command) = request else {
        return Err(CampaignError::Integrity(
            "read stage command kind differs".into(),
        ));
    };
    let checked = match owners.claims.claims.read_claim_admission(command) {
        Ok(checked) => checked,
        Err(error) => {
            return Ok(Some(stage_unresolved_progress(
                campaign,
                locator,
                kind,
                stage.owner_locator.clone(),
                error.to_string(),
                newly.to_vec(),
            )?));
        }
    };
    let claim_index = if kind == StageKind::AdmissionBuilder {
        0
    } else {
        1
    };
    let write_kind = if claim_index == 0 {
        StageKind::EstablishmentBuilder
    } else {
        StageKind::EstablishmentCampaign
    };
    let receipt = committed_receipt(&intent, write_kind)?
        .ok_or_else(|| CampaignError::Conflict("establishment receipt pending".into()))?;
    let selection = campaign
        .owner()?
        .initial_join
        .as_ref()
        .unwrap()
        .selection
        .clone();
    let record = checked_claim_record(
        &checked,
        &intent.selected_claims[claim_index],
        &receipt,
        &intent,
        &selection,
    )?;
    crate::fault_cut(if kind == StageKind::AdmissionBuilder {
        "j1_after_admission_builder_read"
    } else {
        "j1_after_admission_campaign_read"
    });
    let digest = campaign_digest(&record)?;
    if let StageProgress::Checked {
        exact_refs,
        content_digests,
    } = &stage.progress
    {
        if exact_refs.as_slice() != [record.clone()]
            || content_digests.as_slice() != [digest.clone()]
        {
            return Ok(Some(stage_error_progress(
                campaign,
                locator,
                kind,
                "saved checked CE readback differs".into(),
                newly.to_vec(),
            )?));
        }
        return Ok(None);
    }
    let progress = StageProgress::Checked {
        exact_refs: vec![record],
        content_digests: vec![digest],
    };
    match update_stage_progress(campaign, locator, kind, progress)? {
        Effect::Applied { .. } | Effect::Replayed { .. } | Effect::NoEffect(_) => Ok(None),
        Effect::OutcomeUnknown(campaign_locator) => Ok(Some(stage_unresolved_progress(
            campaign,
            locator,
            kind,
            serde_json::to_value(campaign_locator)
                .map_err(|e| CampaignError::Contract(e.to_string()))?,
            "campaign CE readback commit uncertain".into(),
            newly.to_vec(),
        )?)),
    }
}

#[derive(Clone, Debug)]
pub struct InitialEpisodeReadRegistered {
    pub snapshot: Snapshot,
    pub request: EpisodeReadRequest,
    pub registration_receipt: OperationReceipt,
}
fn episode_request_from_stage(
    state: &Snapshot,
    intent: &CompletionIntent,
    stage: &StageIntent,
) -> Result<EpisodeReadRequest> {
    if stage.kind != StageKind::EpisodeResultRead {
        return Err(CampaignError::Integrity(
            "episode stage kind differs".into(),
        ));
    }
    let dispatch = state
        .progress
        .attempts
        .iter()
        .find(|a| a.attempt_id == intent.attempt_id)
        .and_then(|a| a.dispatch.as_ref())
        .ok_or_else(|| CampaignError::Integrity("dispatch absent".into()))?;
    let content_digest = text_field(&stage.command, "contentDigest")?;
    crate::require_sha(&content_digest, "episode content")?;
    let identity_json = json!({"runId":state.identity.run_id,"sliceNumber":state.identity.slice_number,
        "attemptId":state.identity.attempt_id,"planVersion":state.identity.plan_version,
        "reviewObligationId":intent.obligation_id,
        "reviewEpisodeId":dispatch.command.episode_id});
    let identity = review_episode_core::identity::Identity::parse(
        &parse_episode_json(&identity_json.to_string())
            .map_err(|e| CampaignError::Integrity(e.to_string()))?,
    )
    .map_err(|e| CampaignError::Integrity(e.to_string()))?;
    Ok(EpisodeReadRequest::Transition {
        identity,
        transition_id: EpisodeString::new(&dispatch.command.result_transition_id),
        content_digest: EpisodeRevision(content_digest),
    })
}
fn episode_value_from_serde(v: &Value) -> Result<EpisodeValue> {
    parse_episode_json(&v.to_string()).map_err(|e| CampaignError::Contract(e.to_string()))
}
fn serde_from_episode(v: &EpisodeValue) -> Result<Value> {
    serde_json::from_str(&review_episode_core::codec::canonical_json(v))
        .map_err(|e| CampaignError::Contract(e.to_string()))
}
impl Campaign {
    pub fn register_initial_episode_read(
        &mut self,
        identity: &CampaignIdentity,
        expected: &CampaignRevision,
        completion_operation_id: &str,
        registration_operation_id: &str,
        result_content: &EpisodeValue,
    ) -> Result<Effect<InitialEpisodeReadRegistered>> {
        require_text(registration_operation_id, "episode registration operation")?;
        let mut gate = self.owner()?;
        if let Some((receipt, registered)) = gate.store.replay_result(registration_operation_id)? {
            let content = review_episode_core::codec::canonical_json(result_content);
            let intent = registered
                .progress
                .attempts
                .iter()
                .filter_map(|a| a.completion_intent.as_ref())
                .find(|i| i.campaign_operation_id == completion_operation_id)
                .ok_or_else(|| {
                    CampaignError::Conflict("episode registration completion differs".into())
                })?;
            let stage = stage_by_kind(intent, StageKind::EpisodeResultRead).ok_or_else(|| {
                CampaignError::Integrity("episode registration stage absent".into())
            })?;
            if receipt.kind != "register_initial_episode_read"
                || receipt.identity != *identity
                || receipt.prior_revision.as_ref() != Some(expected)
                || stage.registration_operation_id != registration_operation_id
                || stage.command["contentCanonicalJson"] != content
                || stage.request_sha256 != episode_value_digest(result_content)
            {
                return Err(CampaignError::Conflict(
                    "episode registration replay differs".into(),
                ));
            }
            let request = episode_request_from_stage(&registered, intent, stage)?;
            return Ok(Effect::Replayed {
                value: InitialEpisodeReadRegistered {
                    snapshot: registered,
                    request,
                    registration_receipt: receipt.clone(),
                },
                receipt,
            });
        }
        let state = gate.current(identity, expected)?;
        let obligation_id = state
            .progress
            .attempts
            .iter()
            .filter_map(|a| a.completion_intent.as_ref())
            .find(|i| i.campaign_operation_id == completion_operation_id)
            .ok_or_else(|| CampaignError::Conflict("completion intent absent".into()))?
            .obligation_id
            .clone();
        let intent = current_intent(&state, &obligation_id, completion_operation_id)?;
        if stage_by_kind(intent, StageKind::EpisodeResultRead).is_some() {
            return Err(CampaignError::Conflict(
                "episode target already registered".into(),
            ));
        }
        let builder = checked_record_from_stage(
            stage_by_kind(intent, StageKind::AdmissionBuilder)
                .ok_or_else(|| CampaignError::Conflict("builder admission pending".into()))?,
        )?;
        let terminal = checked_record_from_stage(
            stage_by_kind(intent, StageKind::AdmissionCampaign)
                .ok_or_else(|| CampaignError::Conflict("campaign admission pending".into()))?,
        )?;
        let payload = result_content
            .get("payload")
            .ok_or_else(|| CampaignError::Contract("episode payload absent".into()))?;
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
                "episode result content shape differs".into(),
            ));
        }
        let checked = gate
            .initial_join
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("initial join not installed".into()))?
            .evidence
            .read_result(&intent.execution_evidence_ref.checked()?)
            .map_err(|e| CampaignError::Integrity(e.to_string()))?;
        gate.initial_join
            .as_ref()
            .unwrap()
            .selection
            .evidence_profile
            .require_class(checked.evidence_class())
            .map_err(|e| CampaignError::Unsupported(e.to_string()))?;
        let native = parse_episode_json(
            std::str::from_utf8(checked.claim_canonical_bytes())
                .map_err(|_| CampaignError::Contract("native result bytes invalid".into()))?,
        )
        .map_err(|e| CampaignError::Contract(e.to_string()))?;
        ReviewResult::parse(native.clone()).map_err(|e| CampaignError::Contract(e.to_string()))?;
        if payload.get("result") != Some(&native)
            || checked.claim_sha256() != intent.native_result_claim_sha256
            || checked.raw_sha256() != intent.native_result_raw_sha256
        {
            return Err(CampaignError::Conflict(
                "episode content result differs from evidence".into(),
            ));
        }
        review_episode_core::state::validate_text_array(
            payload
                .get("unresolvedQuestions")
                .ok_or_else(|| CampaignError::Contract("unresolved questions absent".into()))?,
            "unresolvedQuestions",
        )
        .map_err(|e| CampaignError::Contract(e.to_string()))?;
        let consumptions = [
            consumption_commitment(&builder, registration_operation_id)?,
            consumption_commitment(&terminal, registration_operation_id)?,
        ];
        let expected_admissions = json!([
            episode_admission(&builder, &consumptions[0]),
            episode_admission(&terminal, &consumptions[1])
        ]);
        let expected_admissions_native = episode_value_from_serde(&expected_admissions)?;
        if payload.get("evidenceAdmissions") != Some(&expected_admissions_native) {
            return Err(CampaignError::Conflict(
                "episode admission pair or consumption differs".into(),
            ));
        }
        for admission in expected_admissions.as_array().unwrap() {
            review_episode_core::state::validate_admission(&episode_value_from_serde(admission)?)
                .map_err(|e| CampaignError::Contract(e.to_string()))?;
        }
        let content_digest = episode_value_digest(result_content);
        let stage_id = stage_id(completion_operation_id, StageKind::EpisodeResultRead);
        let stage = StageIntent {
            stage_id: stage_id.clone(),
            kind: StageKind::EpisodeResultRead,
            registration_operation_id: registration_operation_id.into(),
            registration_base_revision: expected.clone(),
            command: json!({"contentCanonicalJson":review_episode_core::codec::canonical_json(result_content),
                "contentDigest":content_digest,
                "expectedClaimDigest":intent.native_result_claim_sha256,
                "expectedEpisodeDigest":episode_value_digest(&native)}),
            request_sha256: content_digest.clone(),
            original_admission: Value::Null,
            original_admission_sha256: content_digest.clone(),
            owner_selection_digest: intent.owner_selection_digest.clone(),
            owner_locator: json!({"transitionId":state.progress.attempts.iter()
                .find(|a|a.attempt_id==intent.attempt_id).and_then(|a|a.dispatch.as_ref())
                .unwrap().command.result_transition_id,
                "contentDigest":content_digest}),
            evidence_ref: intent.execution_evidence_ref.clone(),
            progress: StageProgress::Registered,
        };
        let digest = campaign_digest(&json!({"identity":identity,"expectedRevision":expected,
            "completionOperationId":completion_operation_id,
            "registrationOperationId":registration_operation_id,"stage":stage,
            "consumptions":consumptions}))?;
        if let Some((receipt, replayed)) = gate.replay(
            registration_operation_id,
            "register_initial_episode_read",
            &digest,
            identity,
        )? {
            let i = current_intent(&replayed, &obligation_id, completion_operation_id)?;
            let s = stage_by_kind(i, StageKind::EpisodeResultRead).ok_or_else(|| {
                CampaignError::Integrity("episode registration absent on replay".into())
            })?;
            let request = episode_request_from_stage(&replayed, i, s)?;
            return Ok(Effect::Replayed {
                value: InitialEpisodeReadRegistered {
                    snapshot: replayed,
                    request,
                    registration_receipt: receipt.clone(),
                },
                receipt,
            });
        }
        let mut next = state.clone();
        let next_intent = current_intent_mut(&mut next, &obligation_id, completion_operation_id)?;
        next_intent.consumption_intents = consumptions.to_vec();
        next_intent.stages.push(stage);
        next.revision = state_revision(&next)?;
        preflight_snapshot(&next)?;
        Ok(
            match gate.put(
                next,
                Some(expected.clone()),
                registration_operation_id,
                "register_initial_episode_read",
                digest,
                false,
                None,
            ) {
                Effect::Applied { value, receipt } => {
                    let i = current_intent(&value, &obligation_id, completion_operation_id)?;
                    let s = stage_by_kind(i, StageKind::EpisodeResultRead).unwrap();
                    let request = episode_request_from_stage(&value, i, s)?;
                    Effect::Applied {
                        value: InitialEpisodeReadRegistered {
                            snapshot: value,
                            request,
                            registration_receipt: receipt.clone(),
                        },
                        receipt,
                    }
                }
                Effect::Replayed { value, receipt } => {
                    let i = current_intent(&value, &obligation_id, completion_operation_id)?;
                    let s = stage_by_kind(i, StageKind::EpisodeResultRead).unwrap();
                    let request = episode_request_from_stage(&value, i, s)?;
                    Effect::Replayed {
                        value: InitialEpisodeReadRegistered {
                            snapshot: value,
                            request,
                            registration_receipt: receipt.clone(),
                        },
                        receipt,
                    }
                }
                Effect::NoEffect(reason) => Effect::NoEffect(reason),
                Effect::OutcomeUnknown(locator) => Effect::OutcomeUnknown(locator),
            },
        )
    }
}

fn read_episode_stage(
    campaign: &mut Campaign,
    locator: &InitialCompletionLocator,
    owners: &mut InitialCompletionOwners<'_>,
    newly: &[Value],
) -> Result<Option<InitialCompletionProgress>> {
    let (state, stage, intent) = current_stage(campaign, locator, StageKind::EpisodeResultRead)?;
    let selected = campaign
        .owner()?
        .initial_join
        .as_ref()
        .unwrap()
        .selection
        .clone();
    let Some(reader) = owners.episode.as_deref_mut() else {
        return Ok(Some(pending(
            campaign,
            locator,
            StageKind::EpisodeResultRead,
            "reader_not_installed",
            newly.to_vec(),
        )?));
    };
    let request = episode_request_from_stage(&state, &intent, &stage)?;
    let readback = match reader.read_checked(&request) {
        Ok(readback) => readback,
        Err(error) => {
            return Ok(Some(stage_unresolved_progress(
                campaign,
                locator,
                StageKind::EpisodeResultRead,
                stage.owner_locator.clone(),
                error.to_string(),
                newly.to_vec(),
            )?));
        }
    };
    if readback.selection() != &selected.episode_selection
        || readback.root() != Path::new(&selected.episode_selection.root)
        || readback.identity() != selected.episode_authority.identity()
        || readback.authority_digest().0 != episode_value_digest(selected.episode_authority.value())
    {
        return Ok(Some(stage_error_progress(
            campaign,
            locator,
            StageKind::EpisodeResultRead,
            "historical episode owner or authority differs".into(),
            newly.to_vec(),
        )?));
    }
    let Some(resolved) = readback.resolved_revision() else {
        return Ok(Some(pending(
            campaign,
            locator,
            StageKind::EpisodeResultRead,
            "exact_target_absent",
            newly.to_vec(),
        )?));
    };
    let result_state = readback
        .state()
        .ok_or_else(|| CampaignError::Integrity("resolved episode state absent".into()))?;
    let result = result_state
        .current_result()
        .map_err(|e| CampaignError::Integrity(e.to_string()))?
        .ok_or_else(|| CampaignError::Conflict("historical episode result absent".into()))?;
    let checked = campaign
        .owner()?
        .initial_join
        .as_ref()
        .unwrap()
        .evidence
        .read_result(&intent.execution_evidence_ref.checked()?)
        .map_err(|e| CampaignError::Integrity(e.to_string()))?;
    selected
        .evidence_profile
        .require_class(checked.evidence_class())
        .map_err(|e| CampaignError::Unsupported(e.to_string()))?;
    let native = parse_episode_json(
        std::str::from_utf8(checked.claim_canonical_bytes())
            .map_err(|_| CampaignError::Integrity("native result bytes invalid".into()))?,
    )
    .map_err(|e| CampaignError::Integrity(e.to_string()))?;
    let expected_content = parse_episode_json(&text_field(&stage.command, "contentCanonicalJson")?)
        .map_err(|e| CampaignError::Integrity(e.to_string()))?;
    let expected_episode_digest = text_field(&stage.command, "expectedEpisodeDigest")?;
    let content_digest = text_field(&stage.command, "contentDigest")?;
    let expected_payload = expected_content
        .get("payload")
        .ok_or_else(|| CampaignError::Integrity("saved episode payload absent".into()))?;
    let expected_admissions = expected_payload
        .get("evidenceAdmissions")
        .ok_or_else(|| CampaignError::Integrity("saved episode admissions absent".into()))?;
    let actual_admissions = result_state.get("evidenceAdmissions");
    if result.value != native
        || expected_payload.get("result") != Some(&native)
        || expected_admissions != actual_admissions
        || episode_value_digest(&native) != expected_episode_digest
        || readback.result_digest().map(|r| r.0.as_str()) != Some(expected_episode_digest.as_str())
        || checked.claim_sha256() != intent.native_result_claim_sha256
        || checked.raw_sha256() != intent.native_result_raw_sha256
    {
        return Ok(Some(stage_error_progress(
            campaign,
            locator,
            StageKind::EpisodeResultRead,
            "historical result or full admission pair differs".into(),
            newly.to_vec(),
        )?));
    }
    let episode_record = json!({
        "root":selected.episode_selection.root,
        "selectionDigest":selected.episode_selection.selection_digest,
        "executableSha256":selected.episode_selection.executable_sha256,
        "authorityDigest":readback.authority_digest().0,
        "identity":serde_from_episode(&readback.identity().value())?,
        "resolvedRevision":resolved.0,
        "observedRevision":readback.observed_revision().map(|r|r.0.clone()),
        "resultTransitionId":stage.owner_locator["transitionId"],
        "contentDigest":content_digest,
        "episodeResultDigest":expected_episode_digest,
        "nativeResultClaimDigest":intent.native_result_claim_sha256,
        "admissionPairDigest":episode_value_digest(expected_admissions),
    });
    let digest = campaign_digest(&episode_record)?;
    crate::fault_cut("j1_after_episode_read");
    if let StageProgress::Checked {
        exact_refs,
        content_digests,
    } = &stage.progress
    {
        // A later native episode transition may advance observedRevision. The
        // first introducing historical state and resolved revision remain fixed.
        let saved = exact_refs
            .first()
            .ok_or_else(|| CampaignError::Integrity("episode checked ref absent".into()))?;
        let mut stable = episode_record.clone();
        stable["observedRevision"] = saved["observedRevision"].clone();
        if exact_refs.len() != 1
            || *saved != stable
            || content_digests.len() != 1
            || content_digests[0] != campaign_digest(&stable)?
        {
            return Ok(Some(stage_error_progress(
                campaign,
                locator,
                StageKind::EpisodeResultRead,
                "saved historical episode readback differs".into(),
                newly.to_vec(),
            )?));
        }
        return Ok(None);
    }
    match update_stage_progress(
        campaign,
        locator,
        StageKind::EpisodeResultRead,
        StageProgress::Checked {
            exact_refs: vec![episode_record],
            content_digests: vec![digest],
        },
    )? {
        Effect::Applied { .. } | Effect::Replayed { .. } | Effect::NoEffect(_) => Ok(None),
        Effect::OutcomeUnknown(campaign_locator) => Ok(Some(stage_unresolved_progress(
            campaign,
            locator,
            StageKind::EpisodeResultRead,
            serde_json::to_value(campaign_locator)
                .map_err(|e| CampaignError::Contract(e.to_string()))?,
            "campaign episode readback commit uncertain".into(),
            newly.to_vec(),
        )?)),
    }
}
fn commit_joined(
    campaign: &mut Campaign,
    locator: &InitialCompletionLocator,
    newly: Vec<Value>,
) -> Result<InitialCompletionProgress> {
    let mut gate = campaign.owner()?;
    let state = gate
        .store
        .read(&locator.identity)?
        .ok_or_else(|| CampaignError::Integrity("campaign absent".into()))?;
    let intent = current_intent(
        &state,
        &locator.obligation_id,
        &locator.completion_operation_id,
    )?;
    let builder = checked_record_from_stage(
        stage_by_kind(intent, StageKind::AdmissionBuilder)
            .ok_or_else(|| CampaignError::Conflict("builder readback absent".into()))?,
    )?;
    let terminal = checked_record_from_stage(
        stage_by_kind(intent, StageKind::AdmissionCampaign)
            .ok_or_else(|| CampaignError::Conflict("campaign readback absent".into()))?,
    )?;
    let episode = checked_record_from_stage(
        stage_by_kind(intent, StageKind::EpisodeResultRead)
            .ok_or_else(|| CampaignError::Conflict("episode readback absent".into()))?,
    )?;
    let joined = JoinedReadbacks {
        builder,
        campaign: terminal,
        episode,
    };
    if let Some(saved) = &intent.joined_readbacks {
        if serde_json::to_value(saved).ok() != serde_json::to_value(&joined).ok() {
            return Err(CampaignError::Conflict(
                "joined readbacks differ from owner".into(),
            ));
        }
        drop(gate);
        return current_progress(
            campaign,
            locator,
            InitialProgressState::OwnerReadsJoined,
            newly,
        );
    }
    let operation_id = format!("{}:joined", locator.completion_operation_id);
    let digest = campaign_digest(&json!({"identity":locator.identity,
        "completionOperationId":locator.completion_operation_id,
        "operationId":operation_id,"joined":joined}))?;
    if let Some((_receipt, replayed)) = gate.replay(
        &operation_id,
        "commit_initial_readbacks",
        &digest,
        &locator.identity,
    )? {
        if current_intent(
            &replayed,
            &locator.obligation_id,
            &locator.completion_operation_id,
        )?
        .joined_readbacks
        .is_none()
        {
            return Err(CampaignError::Integrity(
                "joined replay lacks readbacks".into(),
            ));
        }
        drop(gate);
        return current_progress(
            campaign,
            locator,
            InitialProgressState::OwnerReadsJoined,
            newly,
        );
    }
    let mut next = state.clone();
    current_intent_mut(
        &mut next,
        &locator.obligation_id,
        &locator.completion_operation_id,
    )?
    .joined_readbacks = Some(joined);
    next.revision = state_revision(&next)?;
    preflight_snapshot(&next)?;
    let effect = gate.put(
        next,
        Some(state.revision),
        &operation_id,
        "commit_initial_readbacks",
        digest,
        false,
        None,
    );
    drop(gate);
    match effect {
        Effect::Applied { .. } | Effect::Replayed { .. } => {
            crate::fault_cut("j1_after_joined_commit");
            current_progress(
                campaign,
                locator,
                InitialProgressState::OwnerReadsJoined,
                newly,
            )
        }
        Effect::NoEffect(reason) => stage_error_progress(
            campaign,
            locator,
            StageKind::EpisodeResultRead,
            reason,
            newly,
        ),
        Effect::OutcomeUnknown(campaign_locator) => stage_unresolved_progress(
            campaign,
            locator,
            StageKind::EpisodeResultRead,
            serde_json::to_value(campaign_locator)
                .map_err(|e| CampaignError::Contract(e.to_string()))?,
            "campaign joined-readback commit uncertain".into(),
            newly,
        ),
    }
}

fn validate_completion_locator(
    campaign: &Campaign,
    locator: &InitialCompletionLocator,
) -> Result<(Snapshot, CompletionIntent)> {
    let gate = campaign.owner()?;
    if locator.campaign_root_id != gate.store.root_id
        || locator.anchored_root != gate.store.root
        || locator.identity != gate.config.identity
        || locator.config_digest != gate.config.digest
        || locator.original_registration.root_id != gate.store.root_id
        || locator.original_registration.anchored_root != gate.store.root
        || locator.original_registration.identity != gate.config.identity
        || locator.original_registration.operation_id != locator.completion_operation_id
        || locator.original_registration.kind != "prepare_initial_completion"
        || locator.original_registration.profile_digest != gate.config.digest
    {
        return Err(CampaignError::Conflict(
            "completion locator owner differs".into(),
        ));
    }
    let state = gate
        .store
        .read(&locator.identity)?
        .ok_or_else(|| CampaignError::Integrity("campaign absent".into()))?;
    let intent = current_intent(
        &state,
        &locator.obligation_id,
        &locator.completion_operation_id,
    )?;
    let receipt = gate
        .store
        .receipt(&locator.completion_operation_id)?
        .ok_or_else(|| CampaignError::Conflict("completion registration absent".into()))?;
    if receipt.kind != "prepare_initial_completion"
        || receipt.request_digest != locator.original_registration.request_digest
        || receipt.prior_revision != locator.original_registration.expected_revision
        || receipt.root_id != locator.campaign_root_id
        || intent.attempt_id != locator.attempt_id
        || intent.prepared_request_sha256 != locator.preparation_request_sha256
        || intent.registration_base_revision
            != receipt
                .prior_revision
                .clone()
                .ok_or_else(|| CampaignError::Integrity("registration base absent".into()))?
    {
        return Err(CampaignError::Conflict(
            "completion locator registration differs".into(),
        ));
    }
    let intent = intent.clone();
    Ok((state, intent))
}
fn checked_current_execution(
    campaign: &Campaign,
    locator: &InitialCompletionLocator,
) -> Result<CheckedExecutionResult> {
    let (state, intent) = validate_completion_locator(campaign, locator)?;
    let reference = intent.execution_evidence_ref.checked()?;
    let evidence = Arc::clone(
        &campaign
            .owner()?
            .initial_join
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("initial join not installed".into()))?
            .evidence,
    );
    let checked = evidence
        .read_result(&reference)
        .map_err(|e| CampaignError::Integrity(e.to_string()))?;
    let gate = campaign.owner()?;
    let (dispatch, selected, observation) =
        checked_execution_for_attempt(&gate, &state, &locator.obligation_id, &reference, &checked)?;
    if dispatch != intent.dispatch_operation_id
        || selected
            .iter()
            .map(|s| {
                (
                    &s.claim_id,
                    &s.revision,
                    &s.document_sha256,
                    &s.boundary,
                    &s.consumer,
                )
            })
            .collect::<Vec<_>>()
            != intent
                .selected_claims
                .iter()
                .map(|s| {
                    (
                        &s.claim_id,
                        &s.revision,
                        &s.document_sha256,
                        &s.boundary,
                        &s.consumer,
                    )
                })
                .collect::<Vec<_>>()
        || serde_json::to_value(&observation).ok()
            != serde_json::to_value(&intent.observation_selection).ok()
        || checked.claim_sha256() != intent.native_result_claim_sha256
        || checked.raw_sha256() != intent.native_result_raw_sha256
        || checked.provenance().execution_session_id != intent.execution_session_id
    {
        return Err(CampaignError::Conflict(
            "execution readback changed after preparation".into(),
        ));
    }
    Ok(checked)
}
fn in_memory_unresolved(
    locator: &InitialCompletionLocator,
    last: &Snapshot,
    stage: StageKind,
    reason: String,
    newly: Vec<Value>,
) -> Result<InitialCompletionProgress> {
    let intent = current_intent(
        last,
        &locator.obligation_id,
        &locator.completion_operation_id,
    )?;
    let owner_locator = stage_by_kind(intent, stage)
        .map(|saved| saved.owner_locator.clone())
        .unwrap_or(Value::Null);
    Ok(InitialCompletionProgress {
        locator: locator.clone(),
        last_checked_revision: last.revision.clone(),
        stages: intent.stages.clone(),
        newly_observed_receipts: newly,
        state: InitialProgressState::Unresolved {
            stage,
            owner_locator,
            reason,
        },
    })
}
fn drive_initial(
    campaign: &mut Campaign,
    locator: &InitialCompletionLocator,
    owners: &mut InitialCompletionOwners<'_>,
) -> Result<InitialCompletionProgress> {
    checked_owner_wrapper(campaign, owners)?;
    let (mut last, initial) = validate_completion_locator(campaign, locator)?;
    let checked = checked_current_execution(campaign, locator)?;
    let mut newly = Vec::new();
    let mut kinds = Vec::new();
    macro_rules! checked_step {
        ($expression:expr,$stage:expr) => {
            match $expression {
                Ok(value) => value,
                Err(error) => {
                    return in_memory_unresolved(
                        locator,
                        &last,
                        $stage,
                        error.to_string(),
                        newly.clone(),
                    )
                }
            }
        };
    }
    if matches!(
        initial.observation_selection,
        ObservationSelection::Present { .. }
    ) {
        kinds.push(StageKind::Observation);
    }
    kinds.extend([
        StageKind::EstablishmentBuilder,
        StageKind::EstablishmentCampaign,
        StageKind::AdmissionBuilder,
        StageKind::AdmissionCampaign,
    ]);
    for kind in kinds {
        let (snapshot, intent) =
            checked_step!(validate_completion_locator(campaign, locator), kind);
        last = snapshot;
        if stage_by_kind(&intent, kind).is_none() {
            match checked_step!(
                register_ce_stage(campaign, locator, kind, &checked, owners),
                kind
            ) {
                Effect::Applied { .. } | Effect::Replayed { .. } | Effect::NoEffect(_) => {}
                Effect::OutcomeUnknown(campaign_locator) => {
                    return in_memory_unresolved(
                        locator,
                        &last,
                        kind,
                        format!("campaign stage registration uncertain: {campaign_locator:?}"),
                        newly,
                    );
                }
            }
        }
        let (snapshot, _, _) = checked_step!(current_stage(campaign, locator, kind), kind);
        last = snapshot;
        // Leave enough capacity for the largest bounded CE receipt and checked
        // readback before entering a dependent effect.
        if checked_step!(
            serde_json::to_vec(&last).map_err(|e| CampaignError::Contract(e.to_string())),
            kind
        )
        .len()
            > 900_000
        {
            return Ok(InitialCompletionProgress {
                locator: locator.clone(),
                last_checked_revision: last.revision.clone(),
                stages: current_intent(
                    &last,
                    &locator.obligation_id,
                    &locator.completion_operation_id,
                )?
                .stages
                .clone(),
                newly_observed_receipts: newly,
                state: InitialProgressState::Pending {
                    stage: kind,
                    reason: "campaign_capacity_preflight".into(),
                },
            });
        }
        let result = if matches!(
            kind,
            StageKind::Observation
                | StageKind::EstablishmentBuilder
                | StageKind::EstablishmentCampaign
        ) {
            checked_step!(
                write_stage(campaign, locator, kind, owners, &mut newly),
                kind
            )
        } else {
            checked_step!(read_stage(campaign, locator, kind, owners, &newly), kind)
        };
        if let Some(progress) = result {
            return Ok(progress);
        }
    }
    let (snapshot, intent) = checked_step!(
        validate_completion_locator(campaign, locator),
        StageKind::EpisodeResultRead
    );
    last = snapshot;
    if stage_by_kind(&intent, StageKind::EpisodeResultRead).is_none() {
        return Ok(InitialCompletionProgress {
            locator: locator.clone(),
            last_checked_revision: last.revision.clone(),
            stages: intent.stages.clone(),
            newly_observed_receipts: newly,
            state: InitialProgressState::Pending {
                stage: StageKind::EpisodeResultRead,
                reason: "target_not_registered".into(),
            },
        });
    }
    if let Some(progress) = checked_step!(
        read_episode_stage(campaign, locator, owners, &newly),
        StageKind::EpisodeResultRead
    ) {
        return Ok(progress);
    }
    Ok(checked_step!(
        commit_joined(campaign, locator, newly.clone()),
        StageKind::EpisodeResultRead
    ))
}
impl Campaign {
    /// Reconstruct the exact completion locator after a lost preparation reply.
    /// A locator is descriptive; recovery still rechecks both installed owners.
    pub fn locate_initial_completion(
        &self,
        identity: &CampaignIdentity,
        obligation_id: &str,
        completion_operation_id: &str,
    ) -> Result<Option<InitialCompletionLocator>> {
        let gate = self.owner()?;
        gate.check_identity(identity)?;
        let Some(state) = gate.store.read(identity)? else {
            return Ok(None);
        };
        let Ok(intent) = current_intent(&state, obligation_id, completion_operation_id) else {
            return Ok(None);
        };
        let receipt = gate
            .store
            .receipt(completion_operation_id)?
            .ok_or_else(|| CampaignError::Integrity("completion intent receipt absent".into()))?;
        if receipt.kind != "prepare_initial_completion"
            || receipt.prior_revision.as_ref() != Some(&intent.registration_base_revision)
        {
            return Err(CampaignError::Integrity(
                "completion intent receipt differs".into(),
            ));
        }
        Ok(Some(completion_locator(
            &gate,
            &state,
            obligation_id,
            completion_operation_id,
            receipt.request_digest,
            intent.registration_base_revision.clone(),
        )?))
    }
    pub fn complete_initial(
        &mut self,
        identity: &CampaignIdentity,
        expected: &CampaignRevision,
        obligation_id: &str,
        completion_operation_id: &str,
        owners: &mut InitialCompletionOwners<'_>,
    ) -> Result<InitialCompletionProgress> {
        let locator = {
            let gate = self.owner()?;
            let state = gate.current(identity, expected)?;
            let intent = current_intent(&state, obligation_id, completion_operation_id)?;
            let receipt = gate
                .store
                .receipt(completion_operation_id)?
                .ok_or_else(|| {
                    CampaignError::Integrity("completion registration receipt absent".into())
                })?;
            completion_locator(
                &gate,
                &state,
                obligation_id,
                completion_operation_id,
                receipt.request_digest,
                intent.registration_base_revision.clone(),
            )?
        };
        drive_initial(self, &locator, owners)
    }
    pub fn recover_initial_completion(
        &mut self,
        locator: &InitialCompletionLocator,
        owners: &mut InitialCompletionOwners<'_>,
    ) -> Result<InitialCompletionProgress> {
        validate_completion_locator(self, locator)?;
        drive_initial(self, locator, owners)
    }
}
