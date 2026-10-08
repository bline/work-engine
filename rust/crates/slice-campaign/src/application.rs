use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Value, json};

use crate::admission::{AdmissionHandle, Preparation, PreparedInitial};
use crate::codec::{campaign_digest, raw_sha256};
use crate::contract::{
    AcceptedBoundary, AdvancePhase, Baseline, CampaignIdentity, CampaignRevision, Consequence,
    NativeReviewRequestRef, Obligation, Snapshot, validate_selection,
};
use crate::recovery::{Effect, OperationReceipt, Reconcile, RecoveryLocator};
use crate::store::{Store, state_revision, trusted_digest};
use crate::subject_binding::{verify_existing_subject, verify_gate};
use crate::{CampaignError, Result, require_text};

const SC0_ACCEPTED_PLAN_SHA256: &str =
    "f45b94d3ef985a446002e21a795a51004a5c732e5b150208be09f964152662ef";

/// Constructed at the trusted host boundary, never from a serialized request.
/// The current profile pins the exact SC0 gate capture and historical producer.
pub struct TrustedConfig {
    root: PathBuf,
    repository: PathBuf,
    workspace: PathBuf,
    identity: CampaignIdentity,
    accepted_boundary: AcceptedBoundary,
    baseline: Baseline,
    writer_identity: String,
    gate_capture: Vec<u8>,
    gate_manifest: Vec<u8>,
    gate_receipt: Vec<u8>,
    digest: String,
}

#[derive(Serialize)]
struct ConfigIdentity<'a> {
    repository: &'a Path,
    workspace: &'a Path,
    identity: &'a CampaignIdentity,
    boundary: &'a AcceptedBoundary,
    baseline: &'a Baseline,
    writer_identity: &'a str,
    gate_capture_sha256: String,
    gate_manifest_sha256: String,
    gate_receipt_sha256: String,
}

impl TrustedConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn controlled_sc0(
        root: PathBuf,
        repository: PathBuf,
        workspace: PathBuf,
        identity: CampaignIdentity,
        accepted_boundary: AcceptedBoundary,
        baseline: Baseline,
        writer_identity: String,
        gate_capture: Vec<u8>,
        gate_manifest: Vec<u8>,
        gate_receipt: Vec<u8>,
    ) -> Result<Self> {
        identity.validate()?;
        require_text(&writer_identity, "writer identity")?;
        if accepted_boundary.sha256 != SC0_ACCEPTED_PLAN_SHA256
            || accepted_boundary.reference.trim().is_empty()
        {
            return Err(CampaignError::Unsupported(
                "accepted boundary is outside frozen SC0 profile".into(),
            ));
        }
        for (name, value) in [
            ("acceptedCommit", &baseline.accepted_commit),
            ("acceptedTree", &baseline.accepted_tree),
            ("interSliceCommit", &baseline.inter_slice_commit),
        ] {
            if ![40, 64].contains(&value.len())
                || !value
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            {
                return Err(CampaignError::Contract(format!("{name} invalid Git OID")));
            }
        }
        if !repository.is_absolute()
            || std::fs::canonicalize(&repository).ok().as_deref() != Some(repository.as_path())
        {
            return Err(CampaignError::Root("repository is not anchored".into()));
        }
        if !workspace.is_absolute()
            || std::fs::canonicalize(&workspace).ok().as_deref() != Some(workspace.as_path())
        {
            return Err(CampaignError::Root("workspace is not anchored".into()));
        }
        verify_gate(&gate_capture, &gate_manifest, &gate_receipt)?;
        let observed: Value = serde_json::from_slice(&gate_capture)
            .map_err(|e| CampaignError::Subject(e.to_string()))?;
        if identity.run_id != "sc0-oracle-c2"
            || identity.slice_number != 1
            || identity.plan_version != "sc0-controlled-v1"
            || observed["baseline_commit_oid"] != baseline.accepted_commit
            || observed["baseline_tree_oid"] != baseline.accepted_tree
            || baseline.inter_slice_commit != baseline.accepted_commit
        {
            return Err(CampaignError::Unsupported(
                "campaign identity or baseline differs from frozen SC0 gate profile".into(),
            ));
        }
        let digest = trusted_digest(&ConfigIdentity {
            repository: &repository,
            workspace: &workspace,
            identity: &identity,
            boundary: &accepted_boundary,
            baseline: &baseline,
            writer_identity: &writer_identity,
            gate_capture_sha256: raw_sha256(&gate_capture),
            gate_manifest_sha256: raw_sha256(&gate_manifest),
            gate_receipt_sha256: raw_sha256(&gate_receipt),
        })?;
        Ok(Self {
            root,
            repository,
            workspace,
            identity,
            accepted_boundary,
            baseline,
            writer_identity,
            gate_capture,
            gate_manifest,
            gate_receipt,
            digest,
        })
    }
}

#[derive(Clone, Debug)]
pub struct AdmitRequest {
    pub identity: CampaignIdentity,
    pub repository: PathBuf,
    pub workspace: PathBuf,
    pub accepted_boundary: AcceptedBoundary,
    pub baseline: Baseline,
    pub expected_impact: Option<Consequence>,
}

pub struct Campaign {
    store: Store,
    config: TrustedConfig,
}

impl Campaign {
    pub fn initialize(config: TrustedConfig) -> Result<Self> {
        let store = Store::initialize(&config.root, &config.digest, &config.writer_identity)?;
        Ok(Self { store, config })
    }
    pub fn open(config: TrustedConfig) -> Result<Self> {
        let store = Store::open(&config.root, &config.digest, &config.writer_identity)?;
        Ok(Self { store, config })
    }
    pub fn root_id(&self) -> &str {
        &self.store.root_id
    }
    pub fn anchored_root(&self) -> &Path {
        &self.store.root
    }
    pub fn read(&self, identity: &CampaignIdentity) -> Result<Option<Snapshot>> {
        self.store.read(identity)
    }
    pub fn reconcile_operation(&self, locator: &RecoveryLocator) -> Reconcile {
        self.store.reconcile(locator)
    }

    fn locator(
        &self,
        identity: &CampaignIdentity,
        operation_id: &str,
        kind: &str,
        request_digest: String,
        expected_revision: Option<CampaignRevision>,
    ) -> RecoveryLocator {
        RecoveryLocator {
            root_id: self.store.root_id.clone(),
            anchored_root: self.store.root.clone(),
            identity: identity.clone(),
            operation_id: operation_id.into(),
            kind: kind.into(),
            request_digest,
            expected_revision,
            profile_digest: self.config.digest.clone(),
        }
    }
    fn receipt(
        &self,
        state: &Snapshot,
        prior: Option<CampaignRevision>,
        operation_id: &str,
        kind: &str,
        request_digest: String,
    ) -> OperationReceipt {
        OperationReceipt {
            root_id: self.store.root_id.clone(),
            identity: state.identity.clone(),
            operation_id: operation_id.into(),
            kind: kind.into(),
            request_digest,
            profile_digest: self.config.digest.clone(),
            prior_revision: prior,
            result_revision: state.revision.clone(),
            result_reference: format!(
                "campaign:{}@{}",
                state.identity.key(),
                state.revision.as_str()
            ),
        }
    }
    fn replay(
        &self,
        operation_id: &str,
        kind: &str,
        digest: &str,
        identity: &CampaignIdentity,
    ) -> Result<Option<(OperationReceipt, Snapshot)>> {
        if let Some((receipt, snapshot)) = self.store.replay_result(operation_id)? {
            if receipt.kind != kind
                || receipt.request_digest != digest
                || receipt.identity != *identity
                || receipt.root_id != self.store.root_id
            {
                return Err(CampaignError::Conflict(
                    "operation ID already binds different content".into(),
                ));
            }
            return Ok(Some((receipt, snapshot)));
        }
        Ok(None)
    }
    #[allow(clippy::too_many_arguments)] // One atomic state/receipt/reservation write.
    fn put(
        &mut self,
        state: Snapshot,
        prior: Option<CampaignRevision>,
        operation_id: &str,
        kind: &str,
        digest: String,
        workspace: bool,
        slot: Option<&str>,
    ) -> Effect<Snapshot> {
        let receipt = self.receipt(&state, prior.clone(), operation_id, kind, digest.clone());
        let locator = self.locator(&state.identity, operation_id, kind, digest, prior.clone());
        match self
            .store
            .write(&state, prior.as_ref(), &receipt, workspace, slot)
        {
            Ok(()) => Effect::Applied {
                value: state,
                receipt,
            },
            Err(CampaignError::Conflict(reason)) | Err(CampaignError::Contract(reason)) => {
                Effect::NoEffect(reason)
            }
            Err(_) => Effect::OutcomeUnknown(locator),
        }
    }
    fn current(
        &self,
        identity: &CampaignIdentity,
        expected: &CampaignRevision,
    ) -> Result<Snapshot> {
        let state = self
            .store
            .read(identity)?
            .ok_or_else(|| CampaignError::Conflict("campaign absent".into()))?;
        if &state.revision != expected {
            return Err(CampaignError::Conflict("expected revision differs".into()));
        }
        Ok(state)
    }
    fn check_identity(&self, identity: &CampaignIdentity) -> Result<()> {
        identity.validate()?;
        if identity != &self.config.identity {
            return Err(CampaignError::Contract(
                "campaign identity outside trusted configuration".into(),
            ));
        }
        Ok(())
    }

    pub fn admit(&mut self, operation_id: &str, request: AdmitRequest) -> Result<Effect<Snapshot>> {
        require_text(operation_id, "operation ID")?;
        self.check_identity(&request.identity)?;
        if request.repository != self.config.repository
            || request.workspace != self.config.workspace
            || request.accepted_boundary != self.config.accepted_boundary
            || request.baseline != self.config.baseline
        {
            return Ok(Effect::NoEffect(
                "admission differs from trusted configuration".into(),
            ));
        }
        if let Some(impact) = &request.expected_impact {
            impact.validate()?;
        }
        let digest = campaign_digest(
            &json!({"operationId":operation_id,"identity":request.identity,"repository":request.repository,"workspace":request.workspace,"acceptedBoundary":request.accepted_boundary,"baseline":request.baseline,"expectedImpact":request.expected_impact}),
        )?;
        if let Some((receipt, state)) =
            self.replay(operation_id, "admit", &digest, &request.identity)?
        {
            return Ok(Effect::Replayed {
                value: state,
                receipt,
            });
        }
        if self.store.read(&request.identity)?.is_some() {
            return Ok(Effect::NoEffect("campaign already admitted".into()));
        }
        let mut state = Snapshot {
            schema_version: 1,
            identity: request.identity,
            workspace: request.workspace.to_string_lossy().into_owned(),
            repository: request.repository.to_string_lossy().into_owned(),
            accepted_boundary: request.accepted_boundary,
            baseline: request.baseline,
            expected_impact: request.expected_impact,
            phase: "accepted".into(),
            latest_consequence: None,
            candidate: None,
            candidate_receipt: None,
            physical_profile: None,
            gate_capture_digest: None,
            review_selection: None,
            selection_ref: None,
            obligations: Vec::new(),
            revision: CampaignRevision::new("0".repeat(64))?,
        };
        state.revision = state_revision(&state)?;
        Ok(self.put(state, None, operation_id, "admit", digest, true, None))
    }

    pub fn advance(
        &mut self,
        identity: &CampaignIdentity,
        expected: &CampaignRevision,
        operation_id: &str,
        phase: AdvancePhase,
        consequence: Consequence,
    ) -> Result<Effect<Snapshot>> {
        self.check_identity(identity)?;
        require_text(operation_id, "operation ID")?;
        consequence.validate()?;
        let digest = campaign_digest(
            &json!({"identity":identity,"expectedRevision":expected,"operationId":operation_id,"phase":phase,"consequence":consequence}),
        )?;
        if let Some((receipt, state)) = self.replay(operation_id, "advance", &digest, identity)? {
            return Ok(Effect::Replayed {
                value: state,
                receipt,
            });
        }
        let mut state = self.current(identity, expected)?;
        let allowed = matches!(
            (state.phase.as_str(), &phase),
            ("accepted", AdvancePhase::Implementing)
                | ("implementing", AdvancePhase::GateReady)
                | ("gate_ready", AdvancePhase::ReviewReady)
        );
        if !allowed {
            return Ok(Effect::NoEffect("invalid phase transition".into()));
        }
        if matches!(phase, AdvancePhase::GateReady) {
            let gate = verify_gate(
                &self.config.gate_capture,
                &self.config.gate_manifest,
                &self.config.gate_receipt,
            )?;
            state.gate_capture_digest = Some(gate.capture_digest);
        }
        if matches!(phase, AdvancePhase::ReviewReady)
            && (state.candidate.is_none() || state.physical_profile.is_none())
        {
            return Ok(Effect::NoEffect(
                "review-ready requires verified subject".into(),
            ));
        }
        state.phase = phase.as_str().into();
        state.latest_consequence = Some(consequence);
        state.revision = state_revision(&state)?;
        Ok(self.put(
            state,
            Some(expected.clone()),
            operation_id,
            "advance",
            digest,
            false,
            None,
        ))
    }

    pub fn bind_existing_candidate(
        &mut self,
        identity: &CampaignIdentity,
        expected: &CampaignRevision,
        operation_id: &str,
        candidate_raw: &[u8],
        profile_raw: &[u8],
    ) -> Result<Effect<Snapshot>> {
        self.check_identity(identity)?;
        require_text(operation_id, "operation ID")?;
        let digest = campaign_digest(
            &json!({"identity":identity,"expectedRevision":expected,"operationId":operation_id,"candidateRawSha256":raw_sha256(candidate_raw),"profileRawSha256":raw_sha256(profile_raw)}),
        )?;
        if let Some((receipt, state)) =
            self.replay(operation_id, "bind_existing_candidate", &digest, identity)?
        {
            return Ok(Effect::Replayed {
                value: state,
                receipt,
            });
        }
        let mut state = self.current(identity, expected)?;
        if state.phase != "gate_ready"
            || state.candidate.is_some()
            || state.gate_capture_digest.is_none()
        {
            return Ok(Effect::NoEffect(
                "candidate binding requires unused gate-ready state".into(),
            ));
        }
        let verified = verify_existing_subject(
            &self.config.repository,
            identity,
            &state.baseline,
            candidate_raw,
            profile_raw,
            &self.config.gate_capture,
            &self.config.gate_manifest,
            &self.config.gate_receipt,
        )?;
        state.candidate = Some(verified.reference);
        state.candidate_receipt = Some(verified.candidate_receipt);
        state.physical_profile = Some(verified.physical_profile);
        state.revision = state_revision(&state)?;
        Ok(self.put(
            state,
            Some(expected.clone()),
            operation_id,
            "bind_existing_candidate",
            digest,
            false,
            None,
        ))
    }

    pub fn bind_selection(
        &mut self,
        identity: &CampaignIdentity,
        expected: &CampaignRevision,
        operation_id: &str,
        selection: Value,
    ) -> Result<Effect<Snapshot>> {
        self.check_identity(identity)?;
        require_text(operation_id, "operation ID")?;
        let digest = campaign_digest(
            &json!({"identity":identity,"expectedRevision":expected,"operationId":operation_id,"selection":selection}),
        )?;
        if let Some((receipt, state)) =
            self.replay(operation_id, "bind_selection", &digest, identity)?
        {
            return Ok(Effect::Replayed {
                value: state,
                receipt,
            });
        }
        let mut state = self.current(identity, expected)?;
        if state.phase != "review_ready" || state.review_selection.is_some() {
            return Ok(Effect::NoEffect(
                "selection requires unused review-ready state".into(),
            ));
        }
        let candidate = state
            .candidate
            .as_ref()
            .ok_or_else(|| CampaignError::Integrity("review-ready candidate absent".into()))?;
        let reference = validate_selection(&selection, identity, candidate)?;
        state.obligations = selection["specialists"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| Obligation {
                obligation_id: item["obligationId"].as_str().unwrap().into(),
                skill: item["skill"].as_str().unwrap().into(),
                status: if item["selection"] == "selected" {
                    "pending"
                } else {
                    "omitted"
                }
                .into(),
                request: None,
            })
            .collect();
        state.review_selection = Some(selection);
        state.selection_ref = Some(reference);
        state.revision = state_revision(&state)?;
        Ok(self.put(
            state,
            Some(expected.clone()),
            operation_id,
            "bind_selection",
            digest,
            false,
            None,
        ))
    }

    pub fn prepare_initial(
        &mut self,
        identity: &CampaignIdentity,
        expected: &CampaignRevision,
        obligation_id: &str,
        operation_id: &str,
    ) -> Result<Preparation> {
        self.check_identity(identity)?;
        require_text(operation_id, "operation ID")?;
        require_text(obligation_id, "obligation ID")?;
        let digest = campaign_digest(
            &json!({"identity":identity,"expectedRevision":expected,"obligationId":obligation_id,"operationId":operation_id,"kind":"initial"}),
        )?;
        if let Some((receipt, state)) =
            self.replay(operation_id, "prepare_initial", &digest, identity)?
        {
            let request = state
                .obligations
                .iter()
                .find(|item| item.obligation_id == obligation_id)
                .and_then(|item| item.request.clone())
                .ok_or_else(|| CampaignError::Integrity("prepared request absent".into()))?;
            return Ok(Preparation::Replayed {
                request: Box::new(request),
                receipt,
            });
        }
        let mut state = self.current(identity, expected)?;
        if state.phase != "review_ready" || state.selection_ref.is_none() {
            return Ok(Preparation::NoEffect(
                "initial review requires selection".into(),
            ));
        }
        let Some(index) = state
            .obligations
            .iter()
            .position(|item| item.obligation_id == obligation_id && item.status == "pending")
        else {
            return Ok(Preparation::NoEffect(
                "obligation is not pending and selected".into(),
            ));
        };
        if self.store.slot(identity, obligation_id)?.is_some() {
            return Ok(Preparation::NoEffect(
                "durable request reservation exists".into(),
            ));
        }
        let candidate = state
            .candidate
            .clone()
            .ok_or_else(|| CampaignError::Integrity("candidate absent".into()))?;
        let profile = state
            .physical_profile
            .as_ref()
            .ok_or_else(|| CampaignError::Integrity("profile absent".into()))?;
        let profile_digest = profile["profile_digest"]
            .as_str()
            .ok_or_else(|| CampaignError::Integrity("profile digest absent".into()))?
            .to_owned();
        let selection = state
            .selection_ref
            .clone()
            .ok_or_else(|| CampaignError::Integrity("selection absent".into()))?;
        let mut request = NativeReviewRequestRef {
            root_id: self.store.root_id.clone(),
            identity: identity.clone(),
            obligation_id: obligation_id.into(),
            operation_id: operation_id.into(),
            kind: "initial".into(),
            prepared_revision: expected.clone(),
            request_digest: String::new(),
            selection,
            candidate,
            profile_digest,
        };
        request.request_digest = campaign_digest(
            &json!({"rootId":request.root_id,"identity":request.identity,"obligationId":request.obligation_id,"operationId":request.operation_id,"kind":request.kind,"selection":request.selection,"candidate":request.candidate,"profileDigest":request.profile_digest}),
        )?;
        state.obligations[index].status = "executing".into();
        state.obligations[index].request = Some(request.clone());
        state.revision = state_revision(&state)?;
        request.prepared_revision = state.revision.clone();
        state.obligations[index].request = Some(request.clone());
        let receipt = self.receipt(
            &state,
            Some(expected.clone()),
            operation_id,
            "prepare_initial",
            digest.clone(),
        );
        let locator = self.locator(
            identity,
            operation_id,
            "prepare_initial",
            digest,
            Some(expected.clone()),
        );
        match self
            .store
            .write(&state, Some(expected), &receipt, false, Some(obligation_id))
        {
            Ok(()) => Ok(Preparation::Applied(Box::new(PreparedInitial {
                request: request.clone(),
                handle: AdmissionHandle {
                    owner_epoch: self.store.epoch.clone(),
                    root_id: self.store.root_id.clone(),
                    identity: identity.clone(),
                    obligation_id: obligation_id.into(),
                    operation_id: operation_id.into(),
                    request_digest: request.request_digest.clone(),
                    prepared_revision: state.revision,
                },
                receipt,
            }))),
            Err(CampaignError::Conflict(reason)) | Err(CampaignError::Contract(reason)) => {
                Ok(Preparation::NoEffect(reason))
            }
            Err(_) => Ok(Preparation::OutcomeUnknown(locator)),
        }
    }

    /// Rechecks both the process epoch and durable slot before HP3 may consume
    /// the new request. This consumes the handle; a replay never yields one.
    pub fn consume_initial_admission(
        &self,
        handle: AdmissionHandle,
    ) -> Result<NativeReviewRequestRef> {
        if handle.owner_epoch != self.store.epoch
            || handle.root_id != self.store.root_id
            || handle.identity != self.config.identity
        {
            return Err(CampaignError::Conflict(
                "stale admission owner epoch".into(),
            ));
        }
        let slot = self
            .store
            .slot(&handle.identity, &handle.obligation_id)?
            .ok_or_else(|| CampaignError::Integrity("request slot absent".into()))?;
        if slot
            != (
                handle.operation_id.clone(),
                handle.request_digest.clone(),
                handle.prepared_revision.as_str().to_owned(),
            )
        {
            return Err(CampaignError::Conflict("request slot differs".into()));
        }
        let state = self
            .store
            .read(&handle.identity)?
            .ok_or_else(|| CampaignError::Integrity("campaign absent".into()))?;
        if state.revision != handle.prepared_revision {
            return Err(CampaignError::Conflict(
                "prepared revision is no longer current".into(),
            ));
        }
        let request = state
            .obligations
            .iter()
            .find(|item| item.obligation_id == handle.obligation_id && item.status == "executing")
            .and_then(|item| item.request.clone())
            .ok_or_else(|| CampaignError::Integrity("prepared request absent".into()))?;
        if request.request_digest != handle.request_digest
            || request.prepared_revision != handle.prepared_revision
        {
            return Err(CampaignError::Conflict("prepared request differs".into()));
        }
        Ok(request)
    }

    pub fn read_prepared(
        &self,
        identity: &CampaignIdentity,
        obligation_id: &str,
    ) -> Result<Option<NativeReviewRequestRef>> {
        let state = self.store.read(identity)?;
        Ok(state.and_then(|state| {
            state
                .obligations
                .into_iter()
                .find(|item| item.obligation_id == obligation_id && item.status == "executing")
                .and_then(|item| item.request)
        }))
    }
}
