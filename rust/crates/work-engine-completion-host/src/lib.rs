//! Local completion application route. Trusted startup supplies the actual
//! campaign/claim/episode owner handles; request JSON supplies no authority.
#[cfg(feature = "fixture")]
pub mod fixture_host;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use slice_campaign::publication::CampaignPublication;
use slice_campaign::{CampaignIdentity, InitialCompletionOwners, InitialOutcomeLocator};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use workspace_publication::{AttributedManifest, PublicationRequest, RecordRef, Workspace};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("campaign: {0}")]
    Campaign(#[from] slice_campaign::CampaignError),
    #[error("workspace: {0}")]
    Workspace(#[from] workspace_publication::Error),
    #[error("wire: {0}")]
    Wire(String),
}
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(feature = "test-faults")]
fn fault_cut(name: &str) {
    if std::env::var("WORK_ENGINE_COMPLETION_FAULT_CUT")
        .ok()
        .as_deref()
        != Some(name)
    {
        return;
    }
    let Ok(dir) = std::env::var("WORK_ENGINE_COMPLETION_FAULT_DIR") else {
        return;
    };
    let path = std::path::Path::new(&dir);
    std::fs::write(path.join(format!("{name}.ready")), b"ready").expect("completion cut ready");
    while !path.join(format!("{name}.release")).exists() {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
#[cfg(not(feature = "test-faults"))]
fn fault_cut(_: &str) {}

#[derive(Debug, Deserialize)]
#[serde(
    tag = "command",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[allow(clippy::large_enum_variant)]
pub enum Request {
    InspectCandidate {
        identity: CampaignIdentity,
        locator: InitialOutcomeLocator,
        manifest: AttributedManifest,
    },
    ShowPackage {
        package_ref: RecordRef,
    },
    AcceptCheckpoint {
        package_ref: RecordRef,
        displayed_digest: String,
        request_id: String,
    },
    PreparePublication {
        eligibility_ref: RecordRef,
        target_ref: String,
        message: String,
        request_id: String,
    },
    OpenOffer {
        prepared_ref: RecordRef,
    },
    ShowOffer {
        offer_ref: RecordRef,
    },
    Create {
        offer_ref: RecordRef,
        displayed_digest: String,
        request_id: String,
    },
    Decline {
        offer_ref: RecordRef,
        displayed_digest: String,
        request_id: String,
    },
    Cancel {
        offer_ref: RecordRef,
        displayed_digest: String,
        request_id: String,
    },
    Revoke {
        eligibility_ref: RecordRef,
        request_id: String,
    },
    InspectOperation {
        operation_id: String,
    },
    CloseNoEffect {
        publication_request: PublicationRequest,
    },
    Reconcile {
        publication_request: PublicationRequest,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub ok: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
}

pub struct Application<'a, 'b, 'c> {
    pub campaign: CampaignPublication<'a>,
    pub owners: &'c mut InitialCompletionOwners<'b>,
    pub workspace: Workspace,
}
impl Application<'_, '_, '_> {
    pub fn execute(&mut self, peer: &UnixStream, request: Request) -> Result<Value> {
        match request {
            Request::InspectCandidate {
                identity,
                locator,
                manifest,
            } => {
                let (r, p) = self.campaign.inspect_candidate(
                    &self.workspace,
                    &identity,
                    &locator,
                    self.owners,
                    manifest,
                )?;
                Ok(json!({"packageRef":r,"package":p}))
            }
            Request::ShowPackage { package_ref } => {
                Ok(json!(self.campaign.read_package(&package_ref)?))
            }
            Request::AcceptCheckpoint {
                package_ref,
                displayed_digest,
                request_id,
            } => {
                if package_ref.sha256 != displayed_digest {
                    return Err(Error::Wire("displayed package digest differs".into()));
                }
                let (r, e) = self.campaign.accept_checkpoint_from_peer(
                    peer,
                    &package_ref,
                    &request_id,
                    self.owners,
                )?;
                Ok(json!({"eligibilityRef":r,"eligibility":e}))
            }
            Request::PreparePublication {
                eligibility_ref,
                target_ref,
                message,
                request_id,
            } => {
                let (r, p) = self.workspace.prepare(
                    &self.campaign,
                    &eligibility_ref,
                    &target_ref,
                    &message,
                    &request_id,
                )?;
                Ok(json!({"preparedRef":r,"prepared":p}))
            }
            Request::OpenOffer { prepared_ref } => {
                let (r, o) = self.campaign.open_offer(&self.workspace, &prepared_ref)?;
                Ok(json!({"offerRef":r,"offer":o}))
            }
            Request::ShowOffer { offer_ref } => Ok(json!(self.campaign.read_offer(&offer_ref)?)),
            Request::Create {
                offer_ref,
                displayed_digest,
                request_id,
            } => {
                let offer = self.campaign.read_offer(&offer_ref)?;
                if offer.create_proposal_digest != displayed_digest {
                    return Err(Error::Wire("displayed create proposal differs".into()));
                }
                let decision =
                    self.campaign
                        .decide_from_peer(peer, &offer_ref, "create", &request_id)?;
                let consumption = self.campaign.consume_create_with_status(&offer_ref)?;
                let publication_request = consumption.request;
                let state = self
                    .workspace
                    .inspect(&publication_request.operation_id)?
                    .state;
                let receipt = match state {
                    workspace_publication::OperationState::Prepared
                        if consumption.newly_consumed =>
                    {
                        self.workspace
                            .publish(&self.campaign, &publication_request)?
                    }
                    workspace_publication::OperationState::Prepared => self
                        .workspace
                        .close_no_effect(&self.campaign, &publication_request)?,
                    workspace_publication::OperationState::Admitted => self
                        .workspace
                        .close_no_effect(&self.campaign, &publication_request)?,
                    _ => self
                        .workspace
                        .reconcile(&self.campaign, &publication_request)?,
                };
                if receipt.state == workspace_publication::OperationState::ClosedNoEffect {
                    fault_cut("w1_after_closure_before_settlement");
                }
                let settled = self
                    .campaign
                    .record_result(&self.workspace, &publication_request)?;
                Ok(
                    json!({"decisionRef":decision,"publicationRequest":publication_request,"receipt":receipt,"settled":settled}),
                )
            }
            Request::Decline {
                offer_ref,
                displayed_digest,
                request_id,
            } => {
                let offer = self.campaign.read_offer(&offer_ref)?;
                if offer.create_proposal_digest != displayed_digest {
                    return Err(Error::Wire("displayed offer proposal differs".into()));
                }
                Ok(
                    json!({"decisionRef":self.campaign.decide_from_peer(peer,&offer_ref,"decline",&request_id)?}),
                )
            }
            Request::Cancel {
                offer_ref,
                displayed_digest,
                request_id,
            } => {
                let offer = self.campaign.read_offer(&offer_ref)?;
                if offer.create_proposal_digest != displayed_digest {
                    return Err(Error::Wire("displayed offer proposal differs".into()));
                }
                Ok(
                    json!({"decisionRef":self.campaign.decide_from_peer(peer,&offer_ref,"cancel",&request_id)?}),
                )
            }
            Request::Revoke {
                eligibility_ref,
                request_id,
            } => {
                let outcome =
                    self.campaign
                        .revoke_from_peer(peer, &eligibility_ref, &request_id)?;
                Ok(json!(outcome))
            }
            Request::InspectOperation { operation_id } => {
                Ok(json!(self.workspace.inspect(&operation_id)?))
            }
            Request::CloseNoEffect {
                publication_request,
            } => {
                let receipt = self
                    .workspace
                    .close_no_effect(&self.campaign, &publication_request)?;
                let settled = self
                    .campaign
                    .record_result(&self.workspace, &publication_request)?;
                Ok(json!({"receipt":receipt,"settled":settled}))
            }
            Request::Reconcile {
                publication_request,
            } => {
                let receipt = self
                    .workspace
                    .reconcile(&self.campaign, &publication_request)?;
                let settled = self
                    .campaign
                    .record_result(&self.workspace, &publication_request)?;
                Ok(json!({"receipt":receipt,"settled":settled}))
            }
        }
    }
    /// One bounded JSON command per connected Unix peer. Client half-closes write.
    pub fn serve_connection(&mut self, peer: UnixStream) -> Result<()> {
        self.serve_connection_checked(peer, || Ok(()))
    }
    /// Revalidate deployment authority after the bounded request is read and
    /// before any command can take effect. The connected peer remains intact.
    pub fn serve_connection_checked(
        &mut self,
        mut peer: UnixStream,
        check: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        peer.set_read_timeout(Some(std::time::Duration::from_secs(30)))
            .map_err(|e| Error::Wire(e.to_string()))?;
        let mut bytes = Vec::new();
        (&peer)
            .take(1_048_577)
            .read_to_end(&mut bytes)
            .map_err(|e| Error::Wire(e.to_string()))?;
        if bytes.len() > 1_048_576 {
            return Err(Error::Wire("request exceeds 1 MiB".into()));
        }
        let response = match serde_json::from_slice::<Request>(&bytes) {
            Ok(req) => match check().and_then(|()| self.execute(&peer, req)) {
                Ok(value) => Response {
                    ok: true,
                    result: Some(value),
                    error: None,
                },
                Err(e) => Response {
                    ok: false,
                    result: None,
                    error: Some(e.to_string()),
                },
            },
            Err(e) => Response {
                ok: false,
                result: None,
                error: Some(format!("invalid request: {e}")),
            },
        };
        let mut encoded = serde_json::to_vec(&response).map_err(|e| Error::Wire(e.to_string()))?;
        encoded.push(b'\n');
        peer.write_all(&encoded)
            .map_err(|e| Error::Wire(e.to_string()))
    }
}
