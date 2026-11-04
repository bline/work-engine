//! Publication-only acceptance and separate per-offer operator decisions.
//! The existing campaign's review result remains evidence, never authorization.
use crate::completion::AttemptOutcome;
use crate::contract::{CampaignIdentity, CampaignRevision};
use crate::publication_ingress::authenticated_peer;
use crate::publication_store::PublicationStore;
use crate::{Campaign, CampaignError, Result};
use crate::{
    InitialCompletionOwners, InitialOutcomeKind, InitialOutcomeLocator, InitialOutcomeState,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::os::unix::net::UnixStream;
use std::path::Path;
use workspace_publication::{
    AttributedManifest, AuthorityReadback, CheckedCreate, Eligibility, OperationState,
    PreparedPublication, PublicationCandidate, PublicationReceipt, PublicationRequest, RecordRef,
    Workspace, codec,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InspectionPackage {
    pub candidate: PublicationCandidate,
    pub candidate_digest: String,
    pub campaign_identity: CampaignIdentity,
    pub campaign_revision: CampaignRevision,
    pub review_locator: InitialOutcomeLocator,
    pub native_phase: String,
    pub native_verdict: String,
    pub findings: Value,
    pub limitations: Value,
    pub unresolved_questions: Value,
    pub evidence_summary: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperatorDecision {
    pub action: String,
    pub subject_ref: RecordRef,
    pub subject_revision: u64,
    pub authority_source_digest: String,
    pub observed_peer_uid: u32,
    pub request_id: String,
    pub server_time_ms: u128,
    pub authority_mode: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevocationOutcome {
    pub decision_ref: RecordRef,
    pub revoked: bool,
}
pub struct ConsumptionOutcome {
    pub request: PublicationRequest,
    pub newly_consumed: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompletionOffer {
    pub eligibility_ref: RecordRef,
    pub prepared_ref: RecordRef,
    pub prepared: PreparedPublication,
    pub create_proposal_digest: String,
    pub state: String,
    pub revision: u64,
}

pub struct CampaignPublication<'a> {
    campaign: Option<&'a mut Campaign>,
    store: PublicationStore,
    operator_uid: u32,
    authority_source_digest: String,
    authority_mode: String,
}
type OfferDecisionRow = (Vec<u8>, String, i64, Option<Vec<u8>>, Option<Vec<u8>>);
fn err(e: workspace_publication::Error) -> CampaignError {
    CampaignError::Contract(e.to_string())
}
fn sql<T>(r: rusqlite::Result<T>) -> Result<T> {
    r.map_err(|e| CampaignError::Io(e.to_string()))
}
fn timestamp() -> Result<u128> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| CampaignError::Io(e.to_string()))?
        .as_millis())
}
fn reference(bytes: Vec<u8>) -> Result<RecordRef> {
    serde_json::from_slice(&bytes).map_err(|e| CampaignError::Integrity(e.to_string()))
}
impl<'a> CampaignPublication<'a> {
    pub fn initialize(
        campaign: &'a mut Campaign,
        root: &Path,
        operator_uid: u32,
        authority_source_digest: String,
        fixture_operator: bool,
    ) -> Result<Self> {
        crate::require_sha(&authority_source_digest, "authority source")?;
        if !root.starts_with(campaign.anchored_root()) {
            return Err(CampaignError::Root(
                "completion store must be under campaign owner root".into(),
            ));
        }
        let installed=codec::digest("publication-installation",&json!({"source":authority_source_digest,"operatorUid":operator_uid,"mode":if fixture_operator{"fixture_operator"}else{"production"}})).map_err(err)?;
        let store = PublicationStore::initialize(root, campaign.root_id(), &installed)?;
        Ok(Self {
            campaign: Some(campaign),
            store,
            operator_uid,
            authority_source_digest,
            authority_mode: if fixture_operator {
                "fixture_operator"
            } else {
                "production"
            }
            .into(),
        })
    }
    pub fn open(
        campaign: &'a mut Campaign,
        root: &Path,
        operator_uid: u32,
        authority_source_digest: String,
        fixture_operator: bool,
    ) -> Result<Self> {
        crate::require_sha(&authority_source_digest, "authority source")?;
        if !root.starts_with(campaign.anchored_root()) {
            return Err(CampaignError::Root(
                "completion store must be under campaign owner root".into(),
            ));
        }
        let installed=codec::digest("publication-installation",&json!({"source":authority_source_digest,"operatorUid":operator_uid,"mode":if fixture_operator{"fixture_operator"}else{"production"}})).map_err(err)?;
        let store = PublicationStore::open(root, campaign.root_id(), &installed)?;
        Ok(Self {
            campaign: Some(campaign),
            store,
            operator_uid,
            authority_source_digest,
            authority_mode: if fixture_operator {
                "fixture_operator"
            } else {
                "production"
            }
            .into(),
        })
    }
    /// Open only the installed publication decision ledger. This allows
    /// independent authenticated decision workers to compete under its
    /// SQLite write transaction without opening the campaign's single writer.
    #[cfg(feature = "test-faults")]
    pub fn open_decisions(
        campaign_root: &Path,
        campaign_root_id: &str,
        root: &Path,
        operator_uid: u32,
        authority_source_digest: String,
        fixture_operator: bool,
    ) -> Result<Self> {
        crate::require_sha(&authority_source_digest, "authority source")?;
        if !root.starts_with(campaign_root) {
            return Err(CampaignError::Root(
                "completion store must be under campaign owner root".into(),
            ));
        }
        let mode = if fixture_operator {
            "fixture_operator"
        } else {
            "production"
        };
        let installed = codec::digest(
            "publication-installation",
            &json!({"source":authority_source_digest,"operatorUid":operator_uid,"mode":mode}),
        )
        .map_err(err)?;
        let store = PublicationStore::open(root, campaign_root_id, &installed)?;
        Ok(Self {
            campaign: None,
            store,
            operator_uid,
            authority_source_digest,
            authority_mode: mode.into(),
        })
    }
    pub fn root_id(&self) -> &str {
        &self.store.root_id
    }
    pub fn read_package(&self, r: &RecordRef) -> Result<InspectionPackage> {
        self.store.get(r)
    }
    pub fn read_decision(&self, r: &RecordRef) -> Result<OperatorDecision> {
        self.store.get(r)
    }
    pub fn read_offer(&self, r: &RecordRef) -> Result<CompletionOffer> {
        let mut offer: CompletionOffer = self.store.get(r)?;
        let (state, revision): (String, i64) = sql(self.store.conn.query_row(
            "SELECT state,revision FROM offers WHERE prepared_digest=?1",
            [&offer.prepared_ref.sha256],
            |x| Ok((x.get(0)?, x.get(1)?)),
        ))?;
        offer.state = state;
        offer.revision = revision as u64;
        Ok(offer)
    }
    pub fn inspect_candidate(
        &mut self,
        workspace: &Workspace,
        identity: &CampaignIdentity,
        locator: &InitialOutcomeLocator,
        owners: &mut InitialCompletionOwners<'_>,
        manifest: AttributedManifest,
    ) -> Result<(RecordRef, InspectionPackage)> {
        let campaign = self.campaign.as_deref_mut().ok_or_else(|| {
            CampaignError::Conflict("campaign owner required for inspection".into())
        })?;
        let progress = campaign.recover_initial_outcome(locator, owners)?;
        let view = match progress.state {
            InitialOutcomeState::Committed { view, .. }
                if view.kind == InitialOutcomeKind::Reported =>
            {
                view
            }
            state => {
                return Err(CampaignError::Conflict(format!(
                    "actual review owner readback incomplete or conflicting: {state:?}"
                )));
            }
        };
        let snapshot = campaign
            .read(identity)?
            .ok_or_else(|| CampaignError::Conflict("campaign absent".into()))?;
        let candidate = snapshot
            .candidate
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("candidate absent".into()))?;
        let gate = snapshot
            .gate_capture_digest
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("gate capture absent".into()))?;
        let (intent, _outcome) = snapshot
            .progress
            .attempts
            .iter()
            .filter_map(|a| {
                if !matches!(a.outcome, Some(AttemptOutcome::Completed)) {
                    return None;
                }
                let i = a.completion_intent.as_ref()?;
                let o = i.outcome.as_ref()?;
                Some((i, o))
            })
            .next_back()
            .ok_or_else(|| CampaignError::Conflict("completed owner review absent".into()))?;
        if intent.joined_readbacks.is_none() {
            return Err(CampaignError::Conflict(
                "reported/incomplete review lacks checked outcome".into(),
            ));
        }
        if intent.selected_claims.len() != 2
            || intent
                .selected_claims
                .iter()
                .any(|c| c.document_sha256.len() != 64)
        {
            return Err(CampaignError::Conflict(
                "required claim evidence incomplete".into(),
            ));
        }
        // This profile accepts attribution only through the selected claims
        // just checked by the campaign and review owners. A plausible-looking
        // caller reference cannot install a new claim into the manifest.
        for entry in &manifest.entries {
            let attribution = &entry.attribution_evidence;
            if attribution.kind != "selected-claim"
                || attribution.schema_version != 1
                || attribution.root_id != campaign.root_id()
            {
                return Err(CampaignError::Conflict(
                    "unsupported or foreign selected claim attribution".into(),
                ));
            }
            if !intent.selected_claims.iter().any(|claim| {
                claim.claim_id == attribution.record_id
                    && claim.document_sha256 == attribution.sha256
            }) {
                return Err(CampaignError::Conflict(
                    "required selected claim attribution missing".into(),
                ));
            }
        }
        let review_value =
            serde_json::to_value(&view).map_err(|e| CampaignError::Integrity(e.to_string()))?;
        let review_ref = self.store.put(
            "review-owner-readback",
            &format!(
                "review-{}",
                codec::digest("review-owner-readback", &review_value).map_err(err)?
            ),
            &review_value,
        )?;
        let gate_value = json!({"campaignRootId":campaign.root_id(),"identity":identity,"captureDigest":gate,"candidateTree":candidate.tree,"revision":snapshot.revision});
        let gate_ref = self.store.put(
            "gate-owner-readback",
            &format!(
                "gate-{}",
                codec::digest("gate-owner-readback", &gate_value).map_err(err)?
            ),
            &gate_value,
        )?;
        let candidate_ref = RecordRef {
            kind: "campaign-candidate".into(),
            schema_version: 1,
            root_id: campaign.root_id().into(),
            record_id: candidate.commit.clone(),
            sha256: candidate.receipt_sha256.clone(),
        };
        let campaign_ref = RecordRef {
            kind: "campaign-snapshot".into(),
            schema_version: 1,
            root_id: campaign.root_id().into(),
            record_id: identity.key(),
            sha256: snapshot.revision.as_str().into(),
        };
        let parent = workspace
            .checkpoint_parent(&candidate.commit)
            .map_err(err)?;
        let publication_candidate = PublicationCandidate {
            campaign: campaign_ref,
            candidate: candidate_ref,
            plan_revision: identity.plan_version.clone(),
            scope_revision: snapshot.revision.as_str().into(),
            checkpoint_commit: workspace_publication::GitOid {
                algorithm: "sha1".into(),
                hex: candidate.commit.clone(),
            },
            checkpoint_tree: workspace_publication::GitOid {
                algorithm: "sha1".into(),
                hex: candidate.tree.clone(),
            },
            checkpoint_parent: workspace_publication::GitOid {
                algorithm: "sha1".into(),
                hex: parent,
            },
            review_outcomes: vec![review_ref],
            gate_receipts: vec![gate_ref],
            manifest,
        };
        let candidate_digest =
            codec::digest("publication-candidate", &publication_candidate).map_err(err)?;
        let summary = intent.joined_readbacks.as_ref().unwrap();
        workspace
            .inspect_candidate(&publication_candidate)
            .map_err(err)?;
        let package = InspectionPackage {
            candidate: publication_candidate,
            candidate_digest: candidate_digest.clone(),
            campaign_identity: identity.clone(),
            campaign_revision: snapshot.revision,
            review_locator: locator.clone(),
            native_phase: view.native_phase,
            native_verdict: view.native_verdict,
            findings: serde_json::to_value(&view.findings)
                .map_err(|e| CampaignError::Integrity(e.to_string()))?,
            limitations: serde_json::from_str(&view.limitations_canonical_json)
                .unwrap_or(Value::Null),
            unresolved_questions: serde_json::from_str(&view.unresolved_questions_canonical_json)
                .unwrap_or(Value::Null),
            evidence_summary: json!({"selectedClaims":intent.selected_claims,"joinedReadbacks":summary}),
        };
        let package_ref = self.store.put(
            "inspection-package",
            &format!("package-{candidate_digest}"),
            &package,
        )?;
        sql(self.store.conn.execute(
            "INSERT OR IGNORE INTO packages VALUES(?1,?2,?3)",
            params![
                candidate_digest,
                codec::canonical(&package_ref).map_err(err)?,
                package.campaign_revision.as_str()
            ],
        ))?;
        Ok((package_ref, package))
    }
    pub fn accept_checkpoint_from_peer(
        &mut self,
        peer: &UnixStream,
        package_ref: &RecordRef,
        request_id: &str,
        owners: &mut InitialCompletionOwners<'_>,
    ) -> Result<(RecordRef, Eligibility)> {
        let campaign = self.campaign.as_deref_mut().ok_or_else(|| {
            CampaignError::Conflict("campaign owner required for acceptance".into())
        })?;
        let uid = authenticated_peer(peer, self.operator_uid)?;
        let package: InspectionPackage = self.store.get(package_ref)?;
        let request_digest = codec::digest(
            "accept-request",
            &json!({"package":package_ref,"requestId":request_id}),
        )
        .map_err(err)?;
        if let Some(r) = self.store.replay(request_id, &request_digest)? {
            let e: Eligibility = self.store.get(&r)?;
            return Ok((r, e));
        }
        let fresh = campaign.recover_initial_outcome(&package.review_locator, owners)?;
        let fresh_view = match fresh.state {
            InitialOutcomeState::Committed { view, .. }
                if view.kind == InitialOutcomeKind::Reported =>
            {
                view
            }
            _ => {
                return Err(CampaignError::Conflict(
                    "owner review no longer checked".into(),
                ));
            }
        };
        let fresh_value = serde_json::to_value(&fresh_view)
            .map_err(|e| CampaignError::Integrity(e.to_string()))?;
        if codec::digest("review-owner-readback", &fresh_value).map_err(err)?
            != package.candidate.review_outcomes[0].sha256
        {
            return Err(CampaignError::Conflict(
                "review owner readback changed".into(),
            ));
        }
        let current = campaign
            .read(&package.campaign_identity)?
            .ok_or_else(|| CampaignError::Conflict("campaign disappeared".into()))?;
        if current.revision != package.campaign_revision
            || current.candidate.as_ref().map(|c| c.commit.as_str())
                != Some(package.candidate.checkpoint_commit.hex.as_str())
        {
            return Err(CampaignError::Conflict("inspection package stale".into()));
        }
        let decision = OperatorDecision {
            action: "accept-checkpoint".into(),
            subject_ref: package_ref.clone(),
            subject_revision: 1,
            authority_source_digest: self.authority_source_digest.clone(),
            observed_peer_uid: uid,
            request_id: request_id.into(),
            server_time_ms: timestamp()?,
            authority_mode: self.authority_mode.clone(),
        };
        let tx = self.store.begin_write()?;
        if let Some(r) = self.store.replay_tx(&tx, request_id, &request_digest)? {
            let e: Eligibility = self.store.get(&r)?;
            return Ok((r, e));
        }
        let decision_ref = self.store.put_tx(
            &tx,
            "operator-decision",
            &format!("accept-{request_id}"),
            &decision,
        )?;
        let eligibility = Eligibility {
            candidate_digest: package.candidate_digest.clone(),
            campaign_ref: package.candidate.campaign.clone(),
            acceptance_ref: decision_ref,
            scope: "publication-only".into(),
            active: true,
            revision: 1,
        };
        let eligibility_ref = self.store.put_tx(
            &tx,
            "eligibility",
            &format!("eligibility-{}", package.candidate_digest),
            &eligibility,
        )?;
        sql(tx.execute(
            "INSERT INTO eligibility VALUES(?1,?2,1,1)",
            params![
                package.candidate_digest,
                codec::canonical(&eligibility_ref).map_err(err)?
            ],
        ))?;
        crate::fault_cut("w1_accept_before_replay");
        self.store
            .remember_tx(&tx, request_id, &request_digest, &eligibility_ref)?;
        sql(tx.commit())?;
        crate::fault_cut("w1_accept_after_commit");
        Ok((eligibility_ref, eligibility))
    }
    pub fn revoke_from_peer(
        &self,
        peer: &UnixStream,
        eligibility_ref: &RecordRef,
        request_id: &str,
    ) -> Result<RevocationOutcome> {
        let uid = authenticated_peer(peer, self.operator_uid)?;
        let e: Eligibility = self.store.get(eligibility_ref)?;
        let digest = codec::digest(
            "revoke-request",
            &json!({"eligibility":eligibility_ref,"requestId":request_id}),
        )
        .map_err(err)?;
        let tx = self.store.begin_write()?;
        if let Some(r) = self.store.replay_tx(&tx, request_id, &digest)? {
            let decision: OperatorDecision = self.store.get(&r)?;
            return Ok(RevocationOutcome {
                decision_ref: r,
                revoked: decision.action == "revoke",
            });
        }
        let consumed = {
            let mut stmt = sql(tx.prepare("SELECT record_ref,state FROM offers"))?;
            let rows = sql(stmt.query_map([], |r| {
                Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, String>(1)?))
            }))?;
            let mut found = false;
            for row in rows {
                let (bytes, state) = sql(row)?;
                let offer: CompletionOffer = self.store.get(&reference(bytes)?)?;
                if offer.eligibility_ref == *eligibility_ref
                    && (state == "consumed" || state == "settled")
                {
                    found = true;
                    break;
                }
            }
            found
        };
        let revision: i64 = sql(tx.query_row(
            "SELECT revision FROM eligibility WHERE candidate_digest=?1",
            [&e.candidate_digest],
            |r| r.get(0),
        ))?;
        if !consumed {
            let n=sql(tx.execute("UPDATE eligibility SET active=0,revision=revision+1 WHERE candidate_digest=?1 AND active=1",[&e.candidate_digest]))?;
            if n != 1 {
                return Err(CampaignError::Conflict(
                    "eligibility already revoked".into(),
                ));
            }
        }
        let decision = OperatorDecision {
            action: if consumed {
                "revoke-too-late"
            } else {
                "revoke"
            }
            .into(),
            subject_ref: eligibility_ref.clone(),
            subject_revision: revision as u64,
            authority_source_digest: self.authority_source_digest.clone(),
            observed_peer_uid: uid,
            request_id: request_id.into(),
            server_time_ms: timestamp()?,
            authority_mode: self.authority_mode.clone(),
        };
        let r = self.store.put_tx(
            &tx,
            "operator-decision",
            &format!("revoke-{request_id}"),
            &decision,
        )?;
        crate::fault_cut("w1_revoke_before_replay");
        self.store.remember_tx(&tx, request_id, &digest, &r)?;
        sql(tx.commit())?;
        crate::fault_cut("w1_revoke_after_commit");
        Ok(RevocationOutcome {
            decision_ref: r,
            revoked: !consumed,
        })
    }
    pub fn open_offer(
        &self,
        workspace: &Workspace,
        prepared_ref: &RecordRef,
    ) -> Result<(RecordRef, CompletionOffer)> {
        let prepared: PreparedPublication = workspace.read_record(prepared_ref).map_err(err)?;
        self.check_eligibility(&prepared.eligibility_ref)
            .map_err(|e| CampaignError::Conflict(e.to_string()))?;
        let proposal=codec::digest("create-proposal",&json!({"eligibility":prepared.eligibility_ref,"prepared":prepared_ref,"message":prepared.message,
            "target":prepared.canonical_ref,"parent":prepared.expected_parent,"commit":prepared.prepared_commit,"tree":prepared.integrated_tree})).map_err(err)?;
        let offer = CompletionOffer {
            eligibility_ref: prepared.eligibility_ref.clone(),
            prepared_ref: prepared_ref.clone(),
            prepared,
            create_proposal_digest: proposal,
            state: "open".into(),
            revision: 1,
        };
        let r = self.store.put(
            "completion-offer",
            &format!("offer-{}", prepared_ref.sha256),
            &offer,
        )?;
        sql(self.store.conn.execute(
            "INSERT OR IGNORE INTO offers VALUES(?1,?2,'open',1,NULL,NULL,NULL,NULL)",
            params![prepared_ref.sha256, codec::canonical(&r).map_err(err)?],
        ))?;
        Ok((r, offer))
    }
    pub fn decide_from_peer(
        &self,
        peer: &UnixStream,
        offer_ref: &RecordRef,
        action: &str,
        request_id: &str,
    ) -> Result<RecordRef> {
        let uid = authenticated_peer(peer, self.operator_uid)?;
        if !["create", "decline", "cancel"].contains(&action) {
            return Err(CampaignError::Contract("offer action".into()));
        }
        let req = codec::digest(
            "offer-decision-request",
            &json!({"offer":offer_ref,"action":action,"requestId":request_id}),
        )
        .map_err(err)?;
        let tx = self.store.begin_write()?;
        if let Some(r) = self.store.replay_tx(&tx, request_id, &req)? {
            return Ok(r);
        }
        let offer: CompletionOffer = self.store.get(offer_ref)?;
        let row: (Vec<u8>, String, i64) = sql(tx.query_row(
            "SELECT record_ref,state,revision FROM offers WHERE prepared_digest=?1",
            [&offer.prepared_ref.sha256],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ))?;
        if reference(row.0)? != *offer_ref {
            return Err(CampaignError::Conflict("offer record differs".into()));
        }
        let late = action == "cancel" && (row.1 == "consumed" || row.1 == "settled");
        let cancelable = action == "cancel"
            && ((row.1 == "open" && row.2 == 1) || (row.1 == "create_authorized" && row.2 == 2));
        if !(late || cancelable || row.1 == "open" && row.2 == 1) {
            return Err(CampaignError::Conflict(
                "offer stale or no longer open".into(),
            ));
        }
        if action == "create" {
            self.check_eligibility(&offer.eligibility_ref)
                .map_err(|e| CampaignError::Conflict(e.to_string()))?;
        }
        let decision = OperatorDecision {
            action: if late { "cancel-too-late" } else { action }.into(),
            subject_ref: offer_ref.clone(),
            subject_revision: row.2 as u64,
            authority_source_digest: self.authority_source_digest.clone(),
            observed_peer_uid: uid,
            request_id: request_id.into(),
            server_time_ms: timestamp()?,
            authority_mode: self.authority_mode.clone(),
        };
        let r = self.store.put_tx(
            &tx,
            "operator-decision",
            &format!("decide-{request_id}"),
            &decision,
        )?;
        if !late {
            let encoded = codec::canonical(&r).map_err(err)?;
            let n = if action == "cancel" {
                sql(tx.execute("UPDATE offers SET state='cancelled',revision=revision+1,cancellation_ref=?2 WHERE prepared_digest=?1 AND state=?3 AND revision=?4",
                    params![offer.prepared_ref.sha256,encoded,row.1,row.2]))?
            } else {
                let next = if action == "create" {
                    "create_authorized"
                } else {
                    "declined"
                };
                sql(tx.execute("UPDATE offers SET state=?2,revision=2,decision_ref=?3 WHERE prepared_digest=?1 AND state='open' AND revision=1",
                    params![offer.prepared_ref.sha256,next,encoded]))?
            };
            if n != 1 {
                return Err(CampaignError::Conflict("offer decision CAS lost".into()));
            }
        }
        crate::fault_cut("w1_decision_before_replay");
        self.store.remember_tx(&tx, request_id, &req, &r)?;
        sql(tx.commit())?;
        crate::fault_cut("w1_decision_after_commit");
        Ok(r)
    }
    pub fn consume_create(&self, offer_ref: &RecordRef) -> Result<PublicationRequest> {
        Ok(self.consume_create_with_status(offer_ref)?.request)
    }
    pub fn consume_create_with_status(&self, offer_ref: &RecordRef) -> Result<ConsumptionOutcome> {
        let tx = self.store.begin_write()?;
        let offer: CompletionOffer = self.store.get(offer_ref)?;
        let row:OfferDecisionRow=sql(tx.query_row("SELECT record_ref,state,revision,decision_ref,consumption_ref FROM offers WHERE prepared_digest=?1",[&offer.prepared_ref.sha256],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))))?;
        if reference(row.0)? != *offer_ref {
            return Err(CampaignError::Integrity("offer reference differs".into()));
        }
        if row.1 == "consumed" || row.1 == "settled" {
            return Ok(ConsumptionOutcome {
                request: PublicationRequest {
                    offer_ref: offer_ref.clone(),
                    eligibility_ref: offer.eligibility_ref,
                    prepared_ref: offer.prepared_ref,
                    operation_id: offer.prepared.operation_id,
                },
                newly_consumed: false,
            });
        }
        if row.1 != "create_authorized" || row.2 != 2 {
            return Err(CampaignError::Conflict("create decision absent".into()));
        }
        self.check_eligibility(&offer.eligibility_ref)
            .map_err(|e| CampaignError::Conflict(e.to_string()))?;
        let decision_ref = reference(
            row.3
                .ok_or_else(|| CampaignError::Integrity("decision ref absent".into()))?,
        )?;
        let decision: OperatorDecision = self.store.get(&decision_ref)?;
        if decision.action != "create" || decision.subject_ref != *offer_ref {
            return Err(CampaignError::Integrity("create decision binding".into()));
        }
        let consumption=self.store.put_tx(&tx,"create-consumption",&format!("consume-{}",offer.prepared.operation_id),&json!({"offer":offer_ref,"decision":decision_ref,"operationId":offer.prepared.operation_id}))?;
        let n=sql(tx.execute("UPDATE offers SET state='consumed',revision=3,consumption_ref=?2 WHERE prepared_digest=?1 AND state='create_authorized' AND revision=2",params![offer.prepared_ref.sha256,codec::canonical(&consumption).map_err(err)?]))?;
        if n != 1 {
            return Err(CampaignError::Conflict("consumption CAS lost".into()));
        }
        sql(tx.commit())?;
        crate::fault_cut("w1_consume_after_commit");
        Ok(ConsumptionOutcome {
            request: PublicationRequest {
                offer_ref: offer_ref.clone(),
                eligibility_ref: offer.eligibility_ref,
                prepared_ref: offer.prepared_ref,
                operation_id: offer.prepared.operation_id,
            },
            newly_consumed: true,
        })
    }
    pub fn record_result(
        &self,
        workspace: &Workspace,
        request: &PublicationRequest,
    ) -> Result<PublicationReceipt> {
        let op = workspace.inspect(&request.operation_id).map_err(err)?;
        let result = op
            .result_ref
            .ok_or_else(|| CampaignError::Conflict("workspace result absent".into()))?;
        let receipt: PublicationReceipt = workspace.read_record(&result).map_err(err)?;
        if receipt.operation_id != request.operation_id {
            return Err(CampaignError::Integrity(
                "workspace result operation differs".into(),
            ));
        }
        if receipt.state == OperationState::ClosedNoEffect && receipt.closure_ref.is_none() {
            return Err(CampaignError::Integrity("no-effect closure absent".into()));
        }
        let tx = self.store.begin_write()?;
        self.check_create_consumption(request)
            .map_err(|e| CampaignError::Conflict(e.to_string()))?;
        let r = self.store.put_tx(
            &tx,
            "campaign-publication-result",
            &format!("campaign-result-{}", request.operation_id),
            &json!({"request":request,"workspaceResult":result,"state":receipt.state}),
        )?;
        let offer: CompletionOffer = self.store.get(&request.offer_ref)?;
        let saved: (String, Option<Vec<u8>>) = sql(tx.query_row(
            "SELECT state,result_ref FROM offers WHERE prepared_digest=?1",
            [&offer.prepared_ref.sha256],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ))?;
        match saved.0.as_str() {
            "consumed" => {
                let n=sql(tx.execute("UPDATE offers SET state='settled',revision=revision+1,result_ref=?2 WHERE prepared_digest=?1 AND state='consumed'",params![offer.prepared_ref.sha256,codec::canonical(&r).map_err(err)?]))?;
                if n != 1 {
                    return Err(CampaignError::Conflict("result settlement CAS lost".into()));
                }
            }
            "settled"
                if saved.1.as_deref() == Some(codec::canonical(&r).map_err(err)?.as_slice()) => {}
            _ => return Err(CampaignError::Conflict("offer result state differs".into())),
        }
        sql(tx.commit())?;
        Ok(receipt)
    }
}
impl AuthorityReadback for CampaignPublication<'_> {
    fn check_eligibility(&self, r: &RecordRef) -> workspace_publication::Result<Eligibility> {
        let e: Eligibility = self
            .store
            .get(r)
            .map_err(|x| workspace_publication::Error::Integrity(x.to_string()))?;
        let row: (Vec<u8>, i64, i64) = self
            .store
            .conn
            .query_row(
                "SELECT record_ref,active,revision FROM eligibility WHERE candidate_digest=?1",
                [&e.candidate_digest],
                |x| Ok((x.get(0)?, x.get(1)?, x.get(2)?)),
            )
            .map_err(|x| workspace_publication::Error::Integrity(x.to_string()))?;
        let saved: RecordRef = serde_json::from_slice(&row.0)
            .map_err(|x| workspace_publication::Error::Integrity(x.to_string()))?;
        if saved != *r || row.1 != 1 || row.2 != 1 {
            return Err(workspace_publication::Error::Conflict(
                "eligibility revoked or changed".into(),
            ));
        }
        let decision: OperatorDecision = self
            .store
            .get(&e.acceptance_ref)
            .map_err(|x| workspace_publication::Error::Integrity(x.to_string()))?;
        if decision.action != "accept-checkpoint"
            || decision.observed_peer_uid != self.operator_uid
            || decision.authority_source_digest != self.authority_source_digest
        {
            return Err(workspace_publication::Error::Integrity(
                "operator acceptance binding".into(),
            ));
        }
        Ok(e)
    }
    fn read_candidate(&self, digest: &str) -> workspace_publication::Result<PublicationCandidate> {
        let bytes: Vec<u8> = self
            .store
            .conn
            .query_row(
                "SELECT record_ref FROM packages WHERE candidate_digest=?1",
                [digest],
                |r| r.get(0),
            )
            .map_err(|e| workspace_publication::Error::Integrity(e.to_string()))?;
        let r: RecordRef = serde_json::from_slice(&bytes)
            .map_err(|e| workspace_publication::Error::Integrity(e.to_string()))?;
        let p: InspectionPackage = self
            .store
            .get(&r)
            .map_err(|e| workspace_publication::Error::Integrity(e.to_string()))?;
        if p.candidate_digest != digest
            || codec::digest("publication-candidate", &p.candidate)? != digest
        {
            return Err(workspace_publication::Error::Integrity(
                "candidate digest".into(),
            ));
        }
        Ok(p.candidate)
    }
    fn check_create_consumption(
        &self,
        request: &PublicationRequest,
    ) -> workspace_publication::Result<CheckedCreate> {
        let offer: CompletionOffer = self
            .store
            .get(&request.offer_ref)
            .map_err(|e| workspace_publication::Error::Integrity(e.to_string()))?;
        if offer.prepared_ref != request.prepared_ref
            || offer.eligibility_ref != request.eligibility_ref
            || offer.prepared.operation_id != request.operation_id
        {
            return Err(workspace_publication::Error::Conflict(
                "offer/request binding".into(),
            ));
        }
        let (state, decision, consumption): (String, Option<Vec<u8>>, Option<Vec<u8>>) = self
            .store
            .conn
            .query_row(
                "SELECT state,decision_ref,consumption_ref FROM offers WHERE prepared_digest=?1",
                [&offer.prepared_ref.sha256],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .map_err(|e| workspace_publication::Error::Integrity(e.to_string()))?;
        if state != "consumed" && state != "settled" {
            return Err(workspace_publication::Error::Conflict(
                "create not consumed".into(),
            ));
        }
        let decision: RecordRef =
            serde_json::from_slice(&decision.ok_or_else(|| {
                workspace_publication::Error::Integrity("decision missing".into())
            })?)
            .map_err(|e| workspace_publication::Error::Integrity(e.to_string()))?;
        let d: OperatorDecision = self
            .store
            .get(&decision)
            .map_err(|e| workspace_publication::Error::Integrity(e.to_string()))?;
        if d.action != "create"
            || d.subject_ref != request.offer_ref
            || d.observed_peer_uid != self.operator_uid
        {
            return Err(workspace_publication::Error::Integrity(
                "create decision differs".into(),
            ));
        }
        let consumption: RecordRef = serde_json::from_slice(&consumption.ok_or_else(|| {
            workspace_publication::Error::Integrity("consumption missing".into())
        })?)
        .map_err(|e| workspace_publication::Error::Integrity(e.to_string()))?;
        let c: Value = self
            .store
            .get(&consumption)
            .map_err(|e| workspace_publication::Error::Integrity(e.to_string()))?;
        if c["operationId"] != request.operation_id
            || c["offer"] != serde_json::to_value(&request.offer_ref).unwrap()
            || c["decision"] != serde_json::to_value(&decision).unwrap()
        {
            return Err(workspace_publication::Error::Integrity(
                "consumption binding".into(),
            ));
        }
        Ok(CheckedCreate {
            decision_ref: decision,
            consumption_ref: consumption,
        })
    }
}
