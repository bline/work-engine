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
    #[error("client transport (command outcome may be unknown): {0}")]
    Transport(String),
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
        peer: UnixStream,
        check: impl FnOnce() -> Result<()>,
    ) -> Result<()> {
        serve_command(peer, check, |peer, req| self.execute(peer, req))
    }

    /// Connection-local transport failures never replay a command or stop later
    /// clients. Listener failures still terminate serving.
    pub fn serve_connections_checked(
        &mut self,
        peers: impl IntoIterator<Item = std::io::Result<UnixStream>>,
        mut check: impl FnMut() -> Result<()>,
    ) -> Result<()> {
        serve_connections(peers, |peer| {
            self.serve_connection_checked(peer, &mut check)
        })
    }
}

fn serve_connections(
    peers: impl IntoIterator<Item = std::io::Result<UnixStream>>,
    mut serve: impl FnMut(UnixStream) -> Result<()>,
) -> Result<()> {
    for peer in peers {
        let peer = peer.map_err(|e| Error::Wire(e.to_string()))?;
        match serve(peer) {
            Ok(()) | Err(Error::Transport(_)) => (),
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn serve_command(
    mut peer: UnixStream,
    check: impl FnOnce() -> Result<()>,
    execute: impl FnOnce(&UnixStream, Request) -> Result<Value>,
) -> Result<()> {
    peer.set_write_timeout(Some(std::time::Duration::from_secs(30)))
        .map_err(|e| Error::Transport(e.to_string()))?;
    peer.set_read_timeout(Some(std::time::Duration::from_secs(30)))
        .map_err(|e| Error::Transport(e.to_string()))?;
    let mut bytes = Vec::new();
    (&peer)
        .take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|e| Error::Transport(e.to_string()))?;
    if bytes.len() > 1_048_576 {
        return Err(Error::Transport("request exceeds 1 MiB".into()));
    }
    let response = match serde_json::from_slice::<Request>(&bytes) {
        Ok(req) => match check().and_then(|()| execute(&peer, req)) {
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
    let mut encoded = serde_json::to_vec(&response).map_err(|e| Error::Transport(e.to_string()))?;
    encoded.push(b'\n');
    peer.write_all(&encoded)
        .map_err(|e| Error::Transport(e.to_string()))
}

#[cfg(test)]
mod listener_tests {
    use super::*;
    use std::net::Shutdown;
    use std::os::unix::net::UnixListener;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn oversized_and_disconnected_clients_do_not_stop_listener_or_replay() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("operator.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let executions = AtomicUsize::new(0);
        std::thread::scope(|scope| {
            let server = scope.spawn(|| {
                let mut connection = 0;
                serve_connections(listener.incoming().take(3), |peer| {
                    connection += 1;
                    let result = serve_command(
                        peer,
                        || Ok(()),
                        |peer, req| {
                            // The real connected socket reaches execution unchanged.
                            assert_eq!(
                                rustix::net::sockopt::socket_peercred(peer).unwrap().uid,
                                rustix::process::geteuid()
                            );
                            assert!(matches!(req, Request::InspectOperation { .. }));
                            executions.fetch_add(1, Ordering::SeqCst);
                            Ok(json!({"observed":true}))
                        },
                    );
                    if connection < 3 {
                        assert!(matches!(result, Err(Error::Transport(_))), "{result:?}");
                    }
                    result
                })
                .unwrap();
            });
            let mut oversized = UnixStream::connect(&path).unwrap();
            oversized.write_all(&vec![b'x'; 1_048_577]).unwrap();
            oversized.shutdown(Shutdown::Write).unwrap();
            let mut ignored = Vec::new();
            let _ = oversized.read_to_end(&mut ignored);
            let mut disconnected = UnixStream::connect(&path).unwrap();
            disconnected
                .write_all(br#"{"command":"inspect_operation","operationId":"once"}"#)
                .unwrap();
            disconnected.shutdown(Shutdown::Both).unwrap();
            drop(disconnected);
            let mut valid = UnixStream::connect(&path).unwrap();
            valid
                .write_all(br#"{"command":"inspect_operation","operationId":"later"}"#)
                .unwrap();
            valid.shutdown(Shutdown::Write).unwrap();
            let mut bytes = Vec::new();
            valid.read_to_end(&mut bytes).unwrap();
            let response: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(response["ok"], true);
            assert_eq!(response["result"]["observed"], true);
            server.join().unwrap();
        });
        // A failed response after execution is uncertain, never retried.
        assert_eq!(executions.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn authority_failure_never_executes_and_listener_error_is_fatal() {
        let (server, mut client) = UnixStream::pair().unwrap();
        client
            .write_all(br#"{"command":"inspect_operation","operationId":"denied"}"#)
            .unwrap();
        client.shutdown(Shutdown::Write).unwrap();
        serve_command(
            server,
            || Err(Error::Wire("authority revoked".into())),
            |_, _| panic!("revoked authority executed"),
        )
        .unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(response.contains("authority revoked"));
        assert!(
            serve_connections([Err(std::io::Error::other("listener failed"))], |_| panic!(
                "no peer"
            ))
            .is_err()
        );
    }
}
