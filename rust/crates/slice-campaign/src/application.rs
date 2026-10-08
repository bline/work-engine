use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use claim_evidence::{
    AdmissionBinding, ClaimError, ClaimResult, FindingAdmissionPort, FindingLease,
};
use serde::Serialize;
use serde_json::{Value, json};

use crate::admission::{AdmissionHandle, Preparation, PreparedInitial};
use crate::codec::{campaign_digest, raw_sha256};
use crate::completion::{
    AttemptOutcome, AttemptRecord, CheckedPreEntryFailure, DispatchCommand, DispatchEffect,
    DispatchPermit, DispatchRecord, DispatchedRequest, FailureCustodyRef, RecoveredRequest,
    RecoveryHandle,
};
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

#[cfg(test)]
mod core_tests {
    use super::*;
    use crate::completion::{
        RetryFailureSignature, RetryObligationStatus, RetryProviderEntry, RetryRecoveryFacts,
        RetryRequestFacts,
    };
    #[cfg(feature = "test-faults")]
    use std::time::Duration;

    use crate::test_support as support;

    struct Ready {
        _temp: tempfile::TempDir,
        app: Campaign,
        identity: CampaignIdentity,
        locator: RecoveryLocator,
        _request: NativeReviewRequestRef,
        _command: DispatchCommand,
    }

    fn ready() -> Ready {
        ready_with(false)
    }

    fn ready_with(second_selected: bool) -> Ready {
        let (temp, repo, workspace, identity, baseline, boundary) = support::setup();
        let root = temp.path().join("campaign");
        let mut app = Campaign::initialize(support::config(
            root.clone(),
            repo.clone(),
            workspace.clone(),
            identity.clone(),
            baseline.clone(),
            boundary.clone(),
        ))
        .unwrap();
        let accepted = support::applied(
            app.admit(
                "admit-core",
                AdmitRequest {
                    identity: identity.clone(),
                    repository: repo.clone(),
                    workspace: workspace.clone(),
                    accepted_boundary: boundary.clone(),
                    baseline: baseline.clone(),
                    expected_impact: None,
                },
            )
            .unwrap(),
        );
        let consequence = Consequence {
            owner: "slice-supervisor".into(),
            reference: "accepted-boundary".into(),
            sha256: boundary.sha256.clone(),
        };
        let implementing = support::applied(
            app.advance(
                &identity,
                &accepted.revision,
                "advance-core-1",
                AdvancePhase::Implementing,
                consequence.clone(),
            )
            .unwrap(),
        );
        let gate = support::applied(
            app.advance(
                &identity,
                &implementing.revision,
                "advance-core-2",
                AdvancePhase::GateReady,
                consequence.clone(),
            )
            .unwrap(),
        );
        let (candidate, profile) = support::anchored_artifacts(&repo);
        let bound = support::applied(
            app.bind_existing_candidate(
                &identity,
                &gate.revision,
                "candidate-core",
                &candidate,
                &profile,
            )
            .unwrap(),
        );
        let review = support::applied(
            app.advance(
                &identity,
                &bound.revision,
                "advance-core-3",
                AdvancePhase::ReviewReady,
                consequence,
            )
            .unwrap(),
        );
        let c = review.candidate.as_ref().unwrap();
        let subject = json!({"commit":c.commit,"tree":c.tree,"patchIdentity":c.patch_identity});
        let mut selection = json!({"schemaVersion":2,"owner":"slice-supervisor","selectionId":"selection-core","subject":subject,
            "specialists":[{"obligationId":"review-1","skill":"implementation-review","selection":"selected",
            "requiredClaims":[support::claim(&identity,"review-1",&subject,"builder_projection"),
                support::claim(&identity,"review-1",&subject,"campaign_terminalization")]}]});
        if second_selected {
            selection["specialists"].as_array_mut().unwrap().push(json!({
                "obligationId":"review-2","skill":"implementation-review","selection":"selected",
                "requiredClaims":[support::claim(&identity,"review-2",&subject,"builder_projection"),
                    support::claim(&identity,"review-2",&subject,"campaign_terminalization")]}));
        }
        let selected = support::applied(
            app.bind_selection(&identity, &review.revision, "selection-core", selection)
                .unwrap(),
        );
        #[cfg(feature = "test-faults")]
        let mut fault_manifest = {
            let digest = campaign_digest(
                &json!({"identity":identity,"expectedRevision":selected.revision,
                "obligationId":"review-1","operationId":"prepare-core","kind":"initial"}),
            )
            .unwrap();
            let locator = RecoveryLocator {
                root_id: app.root_id().into(),
                anchored_root: root.clone(),
                identity: identity.clone(),
                operation_id: "prepare-core".into(),
                kind: "prepare_initial".into(),
                request_digest: digest,
                expected_revision: Some(selected.revision.clone()),
                profile_digest: app.owner().unwrap().config.digest.clone(),
            };
            let value = json!({"root":root,"repo":repo,"workspace":workspace,"identity":identity,
                "baseline":baseline,"boundary":boundary,"prepareLocator":locator});
            write_fault_manifest(&value);
            value
        };
        let Preparation::Applied(prepared) = app
            .prepare_initial(&identity, &selected.revision, "review-1", "prepare-core")
            .unwrap()
        else {
            panic!("preparation")
        };
        let request = prepared.request.clone();
        let locator = RecoveryLocator {
            root_id: app.root_id().into(),
            anchored_root: root.clone(),
            identity: identity.clone(),
            operation_id: "prepare-core".into(),
            kind: "prepare_initial".into(),
            request_digest: prepared.receipt.request_digest.clone(),
            expected_revision: Some(selected.revision),
            profile_digest: prepared.receipt.profile_digest.clone(),
        };
        let episode = crate::contract::episode_digest(
            &json!({"identity":identity,"obligationId":"review-1"}),
        )
        .unwrap();
        let command = DispatchCommand {
            episode_id: episode[..32].into(),
            writer_actor: "native-review-host".into(),
            runtime_session: "session-core".into(),
            reviewer_profile: "direct-initial".into(),
            begin_transition_id: "begin-core".into(),
            result_transition_id: "result-core".into(),
        };
        #[cfg(feature = "test-faults")]
        {
            let dispatch_digest = campaign_digest(&json!({"identity":identity,"obligationId":"review-1",
                "preparationOperationId":"prepare-core","requestDigest":request.request_digest,
                "expectedRevision":request.prepared_revision,"operationId":"dispatch-core","command":command})).unwrap();
            let dispatch_locator = RecoveryLocator {
                root_id: app.root_id().into(),
                anchored_root: root.clone(),
                identity: identity.clone(),
                operation_id: "dispatch-core".into(),
                kind: "authorize_dispatch".into(),
                request_digest: dispatch_digest,
                expected_revision: Some(request.prepared_revision.clone()),
                profile_digest: app.owner().unwrap().config.digest.clone(),
            };
            fault_manifest["dispatchLocator"] = serde_json::to_value(dispatch_locator).unwrap();
            write_fault_manifest(&fault_manifest);
        }
        let DispatchEffect::Applied { permit, .. } = app
            .authorize_dispatch(prepared.handle, "dispatch-core", command.clone())
            .unwrap()
        else {
            panic!("dispatch")
        };
        app.consume_dispatch_permit(permit).unwrap();
        Ready {
            _temp: temp,
            app,
            identity,
            locator,
            _request: request,
            _command: command,
        }
    }

    #[test]
    fn definite_pre_entry_retry_retains_prior_attempt_and_slot() {
        let ready = self::ready();
        let mut owner = ready.app.owner().unwrap();
        let state = owner.read(&ready.identity).unwrap().unwrap();
        let Preparation::Applied(prepared) = owner
            .prepare_retry(
                &ready.identity,
                &state.revision,
                "review-1",
                "retry-core",
                "attempt-2",
                CheckedPreEntryFailure::from_owner_facts(
                    ready.identity.attempt_id.clone(),
                    "dispatch-core".into(),
                    "execution-custody-1".into(),
                    "a".repeat(64),
                    RetryObligationStatus::Executing,
                    RetryRecoveryFacts {
                        schema_version: 1,
                        failure_signature: RetryFailureSignature::ProcessStartFailed,
                        provider_entry: RetryProviderEntry::NotEntered,
                        session_available: false,
                        session_id: "session-core".into(),
                        transport_receipt_digest: None,
                        session_artifact_digest: None,
                        error_code: Some("ENOENT".into()),
                        result_digest: None,
                    },
                    None,
                    RetryRequestFacts {
                        retry_session_id: "session-core".into(),
                        continuation_session_id: None,
                        pre_spawn_retry: true,
                    },
                )
                .unwrap(),
            )
            .unwrap()
        else {
            panic!("retry")
        };
        assert_eq!(prepared.request.kind, "retry");
        let state = owner.read(&ready.identity).unwrap().unwrap();
        assert_eq!(state.progress.attempts.len(), 2);
        assert!(matches!(
            state.progress.attempts[0].outcome,
            Some(AttemptOutcome::DefinitePreEntryFailure)
        ));
        assert_eq!(
            state.progress.attempts[0]
                .failure_custody
                .as_ref()
                .unwrap()
                .reference,
            "execution-custody-1"
        );
        assert!(
            owner
                .recover_request(&ready.locator, "review-1")
                .unwrap()
                .preparation_recovery
                .is_none()
        );
        assert_eq!(
            owner
                .store
                .slot(&ready.identity, "review-1")
                .unwrap()
                .unwrap()
                .0,
            "retry-core"
        );
    }

    #[test]
    fn persisted_semantic_tampering_is_refused_on_read_and_replay() {
        fn mutate_state(ready: &Ready, f: impl FnOnce(&mut Snapshot)) {
            let mut state = ready.app.read(&ready.identity).unwrap().unwrap();
            f(&mut state);
            state.revision = state_revision(&state).unwrap();
            let conn =
                rusqlite::Connection::open(ready.app.anchored_root().join("slice-campaign.sqlite"))
                    .unwrap();
            conn.execute(
                "UPDATE campaign_state SET revision=?1,state_json=?2 WHERE identity_key=?3",
                rusqlite::params![
                    state.revision.as_str(),
                    serde_json::to_string(&state).unwrap(),
                    ready.identity.key()
                ],
            )
            .unwrap();
        }
        let ready = self::ready();
        let conn =
            rusqlite::Connection::open(ready.app.anchored_root().join("slice-campaign.sqlite"))
                .unwrap();
        conn.execute(
            "UPDATE campaign_state SET revision=?1 WHERE identity_key=?2",
            rusqlite::params!["0".repeat(64), ready.identity.key()],
        )
        .unwrap();
        assert!(
            ready.app.read(&ready.identity).is_err(),
            "SQL revision drift"
        );

        let ready = self::ready();
        let conn =
            rusqlite::Connection::open(ready.app.anchored_root().join("slice-campaign.sqlite"))
                .unwrap();
        let state = ready.app.read(&ready.identity).unwrap().unwrap();
        let mut value = serde_json::to_value(state).unwrap();
        value["obligations"][0]["request"]["preparedRevision"] = json!("0".repeat(64));
        conn.execute(
            "UPDATE campaign_state SET state_json=?1 WHERE identity_key=?2",
            rusqlite::params![serde_json::to_string(&value).unwrap(), ready.identity.key()],
        )
        .unwrap();
        assert!(
            ready.app.read(&ready.identity).is_err(),
            "excluded prepared revision drift"
        );

        let ready = self::ready();
        mutate_state(&ready, |state| {
            state
                .progress
                .attempts
                .push(state.progress.attempts[0].clone())
        });
        assert!(
            ready.app.read(&ready.identity).is_err(),
            "duplicate attempt identity"
        );

        let ready = self::ready();
        mutate_state(&ready, |state| {
            state.progress.attempts[0]
                .dispatch
                .as_mut()
                .unwrap()
                .may_have_entered = false
        });
        assert!(
            ready.app.read(&ready.identity).is_err(),
            "rehashable dispatch contradiction"
        );

        let ready = self::ready();
        mutate_state(&ready, |state| {
            state.progress.attempts[0].predecessor = Some("other".into())
        });
        assert!(
            ready.app.read(&ready.identity).is_err(),
            "predecessor contradiction"
        );

        let ready = self::ready();
        let conn =
            rusqlite::Connection::open(ready.app.anchored_root().join("slice-campaign.sqlite"))
                .unwrap();
        conn.execute(
            "UPDATE request_slot SET request_digest=?1 WHERE operation_id='prepare-core'",
            ["e".repeat(64)],
        )
        .unwrap();
        assert!(
            ready.app.read(&ready.identity).is_err(),
            "slot request contradiction"
        );

        let ready = self::ready();
        let receipt = ready
            .app
            .owner()
            .unwrap()
            .store
            .receipt("dispatch-core")
            .unwrap()
            .unwrap();
        let locator = RecoveryLocator {
            root_id: ready.app.root_id().into(),
            anchored_root: ready.app.anchored_root().into(),
            identity: ready.identity.clone(),
            operation_id: "dispatch-core".into(),
            kind: "authorize_dispatch".into(),
            request_digest: receipt.request_digest.clone(),
            expected_revision: receipt.prior_revision.clone(),
            profile_digest: receipt.profile_digest.clone(),
        };
        let conn =
            rusqlite::Connection::open(ready.app.anchored_root().join("slice-campaign.sqlite"))
                .unwrap();
        let mut value = serde_json::to_value(receipt).unwrap();
        value["profileDigest"] = json!("f".repeat(64));
        conn.execute(
            "UPDATE operation_receipt SET receipt_json=?1 WHERE operation_id='dispatch-core'",
            [serde_json::to_string(&value).unwrap()],
        )
        .unwrap();
        assert!(
            ready.app.replay_dispatch(&locator).is_err(),
            "receipt profile drift"
        );

        let ready = self::ready();
        let receipt = ready
            .app
            .owner()
            .unwrap()
            .store
            .receipt("dispatch-core")
            .unwrap()
            .unwrap();
        let locator = RecoveryLocator {
            root_id: ready.app.root_id().into(),
            anchored_root: ready.app.anchored_root().into(),
            identity: ready.identity.clone(),
            operation_id: "dispatch-core".into(),
            kind: "authorize_dispatch".into(),
            request_digest: receipt.request_digest.clone(),
            expected_revision: receipt.prior_revision.clone(),
            profile_digest: receipt.profile_digest.clone(),
        };
        let conn =
            rusqlite::Connection::open(ready.app.anchored_root().join("slice-campaign.sqlite"))
                .unwrap();
        let mut value = serde_json::to_value(receipt).unwrap();
        value["resultReference"] = json!("forged-reference");
        conn.execute(
            "UPDATE operation_receipt SET receipt_json=?1 WHERE operation_id='dispatch-core'",
            [serde_json::to_string(&value).unwrap()],
        )
        .unwrap();
        assert!(
            ready.app.replay_dispatch(&locator).is_err(),
            "receipt reference drift"
        );
    }

    #[test]
    fn historical_preparation_replays_after_retry() {
        let ready = self::ready();
        let mut owner = ready.app.owner().unwrap();
        let current = owner.read(&ready.identity).unwrap().unwrap();
        let previous = owner.store.replay_result("prepare-core").unwrap().unwrap();
        assert_eq!(previous.0.kind, "prepare_initial");
        owner
            .prepare_retry(
                &ready.identity,
                &current.revision,
                "review-1",
                "retry-core",
                "attempt-2",
                CheckedPreEntryFailure::from_owner_facts(
                    ready.identity.attempt_id.clone(),
                    "dispatch-core".into(),
                    "execution-custody-1".into(),
                    "a".repeat(64),
                    RetryObligationStatus::Executing,
                    RetryRecoveryFacts {
                        schema_version: 1,
                        failure_signature: RetryFailureSignature::ProcessStartFailed,
                        provider_entry: RetryProviderEntry::NotEntered,
                        session_available: false,
                        session_id: "session-core".into(),
                        transport_receipt_digest: None,
                        session_artifact_digest: None,
                        error_code: Some("ENOENT".into()),
                        result_digest: None,
                    },
                    None,
                    RetryRequestFacts {
                        retry_session_id: "session-core".into(),
                        continuation_session_id: None,
                        pre_spawn_retry: true,
                    },
                )
                .unwrap(),
            )
            .unwrap();
        let replayed = owner.store.replay_result("prepare-core").unwrap().unwrap();
        assert_eq!(replayed.1.revision, previous.1.revision);
        assert_eq!(
            replayed.1.obligations[0]
                .request
                .as_ref()
                .unwrap()
                .prepared_revision,
            previous.1.revision
        );
    }

    #[test]
    fn unsupported_schema_open_does_not_change_database_or_sidecars() {
        use std::fs;
        let ready = self::ready();
        let root = ready.app.anchored_root().to_path_buf();
        let (config_digest, writer_identity) = {
            let owner = ready.app.owner().unwrap();
            (
                owner.config.digest.clone(),
                owner.config.writer_identity.clone(),
            )
        };
        drop(ready.app);
        let database = root.join("slice-campaign.sqlite");
        let conn = rusqlite::Connection::open(&database).unwrap();
        conn.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA user_version=99;")
            .unwrap();
        drop(conn);
        let before = fs::read(&database).unwrap();
        let sidecars = [
            "slice-campaign.sqlite-wal",
            "slice-campaign.sqlite-shm",
            "slice-campaign.sqlite-journal",
        ];
        let inventory = |name: &str| fs::read(root.join(name)).ok();
        let before_sidecars: Vec<_> = sidecars.iter().map(|name| inventory(name)).collect();
        assert!(Store::open(&root, &config_digest, &writer_identity).is_err());
        assert_eq!(
            raw_sha256(&fs::read(&database).unwrap()),
            raw_sha256(&before)
        );
        let after_sidecars: Vec<_> = sidecars.iter().map(|name| inventory(name)).collect();
        assert_eq!(after_sidecars, before_sidecars);
    }

    #[cfg(feature = "test-faults")]
    fn write_fault_manifest(value: &serde_json::Value) {
        use std::io::Write;
        let Ok(path) = std::env::var("SC2_FAULT_MANIFEST") else {
            return;
        };
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(&serde_json::to_vec(value).unwrap()).unwrap();
        file.sync_all().unwrap();
    }

    #[cfg(feature = "test-faults")]
    #[test]
    fn fault_child() {
        if std::env::var_os("SC2_FAULT_CHILD").is_some() {
            let _ = ready();
        }
    }

    #[cfg(feature = "test-faults")]
    #[test]
    fn preparation_and_dispatch_process_cuts() {
        use std::process::{Command, Stdio};
        use std::time::Instant;
        for (cut, prep_committed, dispatch_committed) in [
            ("before_commit_prepare_initial", false, false),
            ("after_commit_prepare_initial", true, false),
            ("before_commit_authorize_dispatch", true, false),
            ("after_commit_authorize_dispatch", true, true),
        ] {
            let scratch = tempfile::tempdir().unwrap();
            let manifest_path = scratch.path().join("case.json");
            let mut child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "application::core_tests::fault_child",
                    "--nocapture",
                    "--test-threads=1",
                ])
                .env("SC2_FAULT_CHILD", "1")
                .env("SC2_FAULT_MANIFEST", &manifest_path)
                .env("SLICE_CAMPAIGN_FAULT_CUT", cut)
                .env("SLICE_CAMPAIGN_FAULT_DIR", scratch.path())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let ready_path = scratch.path().join(format!("{cut}.ready"));
            let until = Instant::now() + Duration::from_secs(10);
            while !ready_path.exists() {
                assert!(Instant::now() < until, "fault child did not reach {cut}");
                assert!(
                    child.try_wait().unwrap().is_none(),
                    "fault child exited before {cut}"
                );
                std::thread::sleep(Duration::from_millis(10));
            }
            child.kill().unwrap();
            child.wait().unwrap();
            let manifest: Value =
                serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
            let path = |name: &str| PathBuf::from(manifest[name].as_str().unwrap());
            let identity: CampaignIdentity =
                serde_json::from_value(manifest["identity"].clone()).unwrap();
            let prepare: RecoveryLocator =
                serde_json::from_value(manifest["prepareLocator"].clone()).unwrap();
            let app = Campaign::open(support::config(
                path("root"),
                path("repo"),
                path("workspace"),
                identity.clone(),
                serde_json::from_value(manifest["baseline"].clone()).unwrap(),
                serde_json::from_value(manifest["boundary"].clone()).unwrap(),
            ))
            .unwrap();
            let prep_status = app.reconcile_operation(&prepare);
            assert_eq!(
                matches!(prep_status, Reconcile::Committed(_)),
                prep_committed,
                "{cut}"
            );
            if prep_committed {
                let recovered = app.recover_request(&prepare, "review-1").unwrap();
                assert_eq!(
                    recovered.preparation_recovery.is_some(),
                    !dispatch_committed,
                    "{cut}"
                );
                assert_eq!(recovered.dispatch.is_some(), dispatch_committed, "{cut}");
            } else {
                assert!(app.read_prepared(&identity, "review-1").unwrap().is_none());
            }
            if let Some(value) = manifest.get("dispatchLocator") {
                let dispatch: RecoveryLocator = serde_json::from_value(value.clone()).unwrap();
                assert_eq!(
                    matches!(app.reconcile_operation(&dispatch), Reconcile::Committed(_)),
                    dispatch_committed,
                    "{cut}"
                );
                if dispatch_committed {
                    assert!(app.replay_dispatch(&dispatch).unwrap().0.may_have_entered);
                }
            }
        }
    }
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

struct CampaignInner {
    store: Store,
    config: TrustedConfig,
    initial_join: Option<InitialJoinInstallation>,
}

/// The store and all admission checks share one process-local gate. A future
/// claims lease holds this gate across its bounded owner operation.
pub struct Campaign {
    inner: Arc<Mutex<CampaignInner>>,
    root_id: String,
    root: PathBuf,
}

/// A campaign-owned port sharing the exact owner gate with all campaign writes.
/// It grants a bounded CE2 lease only for a registered current finding child.
pub struct CampaignClaimsAdmission {
    inner: Arc<Mutex<CampaignInner>>,
}

struct CampaignFindingLease<'a> {
    _gate: MutexGuard<'a, CampaignInner>,
}
impl FindingLease for CampaignFindingLease<'_> {}

impl FindingAdmissionPort for CampaignClaimsAdmission {
    fn acquire<'a>(
        &'a self,
        binding: &AdmissionBinding,
    ) -> ClaimResult<Box<dyn FindingLease + 'a>> {
        let owner = self
            .inner
            .lock()
            .map_err(|_| ClaimError::new("campaign owner gate poisoned"))?;
        owner
            .check_finding_admission(binding)
            .map_err(|e| ClaimError::new(e.to_string()))?;
        Ok(Box::new(CampaignFindingLease { _gate: owner }))
    }
}

impl Campaign {
    pub fn initialize(config: TrustedConfig) -> Result<Self> {
        let store = Store::initialize(&config.root, &config.digest, &config.writer_identity)?;
        Ok(Self::with_store(store, config))
    }
    pub fn open(config: TrustedConfig) -> Result<Self> {
        let store = Store::open(&config.root, &config.digest, &config.writer_identity)?;
        Ok(Self::with_store(store, config))
    }
    fn with_store(store: Store, config: TrustedConfig) -> Self {
        Self {
            root_id: store.root_id.clone(),
            root: store.root.clone(),
            inner: Arc::new(Mutex::new(CampaignInner {
                store,
                config,
                initial_join: None,
            })),
        }
    }
    fn owner(&self) -> Result<MutexGuard<'_, CampaignInner>> {
        self.inner
            .lock()
            .map_err(|_| CampaignError::Integrity("campaign owner gate poisoned".into()))
    }
    pub fn claims_admission(&self) -> CampaignClaimsAdmission {
        CampaignClaimsAdmission {
            inner: Arc::clone(&self.inner),
        }
    }
    pub fn root_id(&self) -> &str {
        &self.root_id
    }
    pub fn anchored_root(&self) -> &Path {
        &self.root
    }
    pub fn read(&self, identity: &CampaignIdentity) -> Result<Option<Snapshot>> {
        self.owner()?.read(identity)
    }
    pub fn reconcile_operation(&self, locator: &RecoveryLocator) -> Reconcile {
        match self.owner() {
            Ok(owner) => owner.reconcile_operation(locator),
            Err(error) => Reconcile::Unresolved(error.to_string()),
        }
    }
    pub fn admit(&mut self, operation_id: &str, request: AdmitRequest) -> Result<Effect<Snapshot>> {
        self.owner()?.admit(operation_id, request)
    }
    pub fn advance(
        &mut self,
        identity: &CampaignIdentity,
        expected: &CampaignRevision,
        operation_id: &str,
        phase: AdvancePhase,
        consequence: Consequence,
    ) -> Result<Effect<Snapshot>> {
        self.owner()?
            .advance(identity, expected, operation_id, phase, consequence)
    }
    pub fn bind_existing_candidate(
        &mut self,
        identity: &CampaignIdentity,
        expected: &CampaignRevision,
        operation_id: &str,
        candidate_raw: &[u8],
        profile_raw: &[u8],
    ) -> Result<Effect<Snapshot>> {
        self.owner()?.bind_existing_candidate(
            identity,
            expected,
            operation_id,
            candidate_raw,
            profile_raw,
        )
    }
    pub fn bind_selection(
        &mut self,
        identity: &CampaignIdentity,
        expected: &CampaignRevision,
        operation_id: &str,
        selection: Value,
    ) -> Result<Effect<Snapshot>> {
        self.owner()?
            .bind_selection(identity, expected, operation_id, selection)
    }
    pub fn prepare_initial(
        &mut self,
        identity: &CampaignIdentity,
        expected: &CampaignRevision,
        obligation_id: &str,
        operation_id: &str,
    ) -> Result<Preparation> {
        self.owner()?
            .prepare_initial(identity, expected, obligation_id, operation_id)
    }
    pub fn consume_initial_admission(
        &self,
        handle: AdmissionHandle,
    ) -> Result<NativeReviewRequestRef> {
        self.owner()?.consume_initial_admission(handle)
    }
    pub fn read_prepared(
        &self,
        identity: &CampaignIdentity,
        obligation_id: &str,
    ) -> Result<Option<NativeReviewRequestRef>> {
        self.owner()?.read_prepared(identity, obligation_id)
    }
    pub fn authorize_dispatch(
        &mut self,
        handle: AdmissionHandle,
        operation_id: &str,
        command: DispatchCommand,
    ) -> Result<DispatchEffect> {
        self.owner()?
            .authorize_dispatch(handle, operation_id, command)
    }
    pub fn authorize_recovered_dispatch(
        &mut self,
        handle: RecoveryHandle,
        operation_id: &str,
        command: DispatchCommand,
    ) -> Result<DispatchEffect> {
        self.owner()?
            .authorize_recovered_dispatch(handle, operation_id, command)
    }
    pub fn consume_dispatch_permit(&self, permit: DispatchPermit) -> Result<DispatchedRequest> {
        self.owner()?.consume_dispatch_permit(permit)
    }
    pub fn recover_request(
        &self,
        locator: &RecoveryLocator,
        obligation_id: &str,
    ) -> Result<RecoveredRequest> {
        self.owner()?.recover_request(locator, obligation_id)
    }
    pub fn replay_dispatch(
        &self,
        locator: &RecoveryLocator,
    ) -> Result<(DispatchRecord, OperationReceipt)> {
        self.owner()?.replay_dispatch(locator)
    }
}

include!("owner_readback.rs");

impl CampaignInner {
    fn check_finding_admission(&self, _binding: &AdmissionBinding) -> Result<()> {
        Err(CampaignError::Unsupported(
            "J2 finding and reliance admission is not enabled in schema 3".into(),
        ))
    }
    fn read(&self, identity: &CampaignIdentity) -> Result<Option<Snapshot>> {
        self.store.read(identity)
    }
    fn reconcile_operation(&self, locator: &RecoveryLocator) -> Reconcile {
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
            schema_version: 3,
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
            progress: Default::default(),
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
        state.progress.attempts.push(AttemptRecord {
            attempt_id: identity.attempt_id.clone(),
            preparation_operation_id: operation_id.into(),
            request_digest: request.request_digest.clone(),
            predecessor: None,
            dispatch: None,
            completion_intent: None,
            outcome: None,
            failure_custody: None,
        });
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
                    prepared_revision: state.revision.clone(),
                    expected_revision: state.revision,
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

    fn checked_preparation(&self, handle: &AdmissionHandle) -> Result<NativeReviewRequestRef> {
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
        let state = self.current(&handle.identity, &handle.expected_revision)?;
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
        let attempt = state
            .progress
            .attempts
            .iter()
            .find(|item| item.preparation_operation_id == handle.operation_id)
            .ok_or_else(|| CampaignError::Integrity("attempt absent".into()))?;
        if attempt.preparation_operation_id != handle.operation_id
            || attempt.request_digest != handle.request_digest
            || attempt.dispatch.is_some()
            || attempt.outcome.is_some()
        {
            return Err(CampaignError::Conflict("active attempt differs".into()));
        }
        if self
            .store
            .dispatch_slot(&handle.identity, &handle.obligation_id)?
            != Some((None, false))
        {
            return Err(CampaignError::Conflict(
                "dispatch already authorized".into(),
            ));
        }
        Ok(request)
    }

    fn authorize_dispatch(
        &mut self,
        handle: AdmissionHandle,
        operation_id: &str,
        command: DispatchCommand,
    ) -> Result<DispatchEffect> {
        require_text(operation_id, "dispatch operation ID")?;
        for (name, value) in [
            ("episode ID", &command.episode_id),
            ("writer actor", &command.writer_actor),
            ("runtime session", &command.runtime_session),
            ("reviewer profile", &command.reviewer_profile),
            ("begin transition", &command.begin_transition_id),
            ("result transition", &command.result_transition_id),
        ] {
            require_text(value, name)?;
        }
        if command.begin_transition_id == command.result_transition_id
            || operation_id == handle.operation_id
        {
            return Ok(DispatchEffect::NoEffect(
                "dispatch identities overlap".into(),
            ));
        }
        let expected_episode = crate::contract::episode_digest(
            &json!({"identity":handle.identity,"obligationId":handle.obligation_id}),
        )?;
        if command.episode_id != expected_episode[..32] {
            return Ok(DispatchEffect::NoEffect("episode identity differs".into()));
        }
        let digest = campaign_digest(&json!({
            "identity":handle.identity,"obligationId":handle.obligation_id,
            "preparationOperationId":handle.operation_id,"requestDigest":handle.request_digest,
            "expectedRevision":handle.expected_revision,"operationId":operation_id,"command":command,
        }))?;
        if let Some((receipt, state)) = self.replay(
            operation_id,
            "authorize_dispatch",
            &digest,
            &handle.identity,
        )? {
            let record = state
                .progress
                .attempts
                .iter()
                .find(|attempt| attempt.preparation_operation_id == handle.operation_id)
                .and_then(|attempt| attempt.dispatch.clone())
                .filter(|record| {
                    record.operation_id == operation_id
                        && record.request_digest == handle.request_digest
                })
                .ok_or_else(|| {
                    CampaignError::Integrity("replayed dispatch record differs".into())
                })?;
            return Ok(DispatchEffect::Replayed { record, receipt });
        }
        let _request = self.checked_preparation(&handle)?;
        let mut state = self.current(&handle.identity, &handle.expected_revision)?;
        let record = DispatchRecord {
            operation_id: operation_id.into(),
            request_digest: handle.request_digest.clone(),
            command: command.clone(),
            may_have_entered: true,
        };
        state
            .progress
            .attempts
            .iter_mut()
            .find(|attempt| attempt.preparation_operation_id == handle.operation_id)
            .ok_or_else(|| CampaignError::Integrity("prepared attempt absent".into()))?
            .dispatch = Some(record);
        state.revision = state_revision(&state)?;
        let result = self.put(
            state,
            Some(handle.expected_revision),
            operation_id,
            "authorize_dispatch",
            digest,
            false,
            None,
        );
        Ok(match result {
            Effect::Applied { value, receipt } => DispatchEffect::Applied {
                permit: DispatchPermit {
                    owner_epoch: handle.owner_epoch,
                    root_id: handle.root_id,
                    identity: handle.identity,
                    obligation_id: handle.obligation_id,
                    request_digest: handle.request_digest,
                    dispatch_operation_id: operation_id.into(),
                    dispatched_revision: value.revision,
                    command,
                },
                receipt,
            },
            Effect::Replayed { .. } => unreachable!("put never replays"),
            Effect::NoEffect(reason) => DispatchEffect::NoEffect(reason),
            Effect::OutcomeUnknown(locator) => DispatchEffect::OutcomeUnknown(locator),
        })
    }

    fn authorize_recovered_dispatch(
        &mut self,
        handle: RecoveryHandle,
        operation_id: &str,
        command: DispatchCommand,
    ) -> Result<DispatchEffect> {
        if handle.owner_epoch != self.store.epoch || handle.root_id != self.store.root_id {
            return Err(CampaignError::Conflict("stale recovery owner epoch".into()));
        }
        let slot = self
            .store
            .slot(&handle.identity, &handle.obligation_id)?
            .ok_or_else(|| CampaignError::Integrity("request slot absent".into()))?;
        if slot.1 != handle.request_digest
            || self
                .store
                .dispatch_slot(&handle.identity, &handle.obligation_id)?
                != Some((None, false))
        {
            return Err(CampaignError::Conflict("recovery slot differs".into()));
        }
        self.authorize_dispatch(
            AdmissionHandle {
                owner_epoch: handle.owner_epoch,
                root_id: handle.root_id,
                identity: handle.identity,
                obligation_id: handle.obligation_id,
                operation_id: slot.0,
                request_digest: handle.request_digest,
                prepared_revision: CampaignRevision::new(slot.2)?,
                expected_revision: handle.current_revision,
            },
            operation_id,
            command,
        )
    }

    fn recover_request(
        &self,
        locator: &RecoveryLocator,
        obligation_id: &str,
    ) -> Result<RecoveredRequest> {
        if !["prepare_initial", "prepare_retry"].contains(&locator.kind.as_str())
            || locator.identity != self.config.identity
        {
            return Err(CampaignError::Contract(
                "recovery locator kind or identity differs".into(),
            ));
        }
        match self.store.reconcile(locator) {
            Reconcile::Committed(_) => {}
            Reconcile::Absent => return Err(CampaignError::Conflict("preparation absent".into())),
            Reconcile::Conflicting(_) => {
                return Err(CampaignError::Conflict(
                    "preparation operation differs".into(),
                ));
            }
            Reconcile::Unresolved(reason) => return Err(CampaignError::Integrity(reason)),
        }
        let (_, historical) = self
            .store
            .replay_result(&locator.operation_id)?
            .ok_or_else(|| CampaignError::Integrity("preparation receipt absent".into()))?;
        let request = historical
            .obligations
            .iter()
            .find(|item| item.obligation_id == obligation_id)
            .and_then(|item| item.request.clone())
            .filter(|request| request.operation_id == locator.operation_id)
            .ok_or_else(|| CampaignError::Conflict("prepared obligation differs".into()))?;
        let slot = self
            .store
            .slot_for_operation(&locator.identity, &request.operation_id)?
            .ok_or_else(|| CampaignError::Integrity("request slot absent".into()))?;
        if slot.obligation_id != obligation_id
            || slot.request_digest != request.request_digest
            || slot.prepared_revision != request.prepared_revision.as_str()
        {
            return Err(CampaignError::Integrity(
                "request slot differs from history".into(),
            ));
        }
        let current = self
            .store
            .read(&locator.identity)?
            .ok_or_else(|| CampaignError::Integrity("campaign absent".into()))?;
        let attempt = current
            .progress
            .attempts
            .iter()
            .find(|item| item.preparation_operation_id == request.operation_id)
            .ok_or_else(|| CampaignError::Integrity("attempt history absent".into()))?;
        if attempt.request_digest != request.request_digest {
            return Err(CampaignError::Integrity("attempt request differs".into()));
        }
        if (slot.dispatch_operation_id.clone(), slot.may_have_entered)
            != (
                attempt
                    .dispatch
                    .as_ref()
                    .map(|item| item.operation_id.clone()),
                attempt.dispatch.is_some(),
            )
        {
            return Err(CampaignError::Integrity(
                "dispatch slot/history differs".into(),
            ));
        }
        let preparation_recovery = if slot.active
            && attempt.dispatch.is_none()
            && current
                .obligations
                .iter()
                .find(|item| item.obligation_id == obligation_id)
                .and_then(|item| item.request.as_ref())
                == Some(&request)
        {
            Some(RecoveryHandle {
                owner_epoch: self.store.epoch.clone(),
                root_id: self.store.root_id.clone(),
                identity: locator.identity.clone(),
                obligation_id: obligation_id.into(),
                request_digest: request.request_digest.clone(),
                current_revision: current.revision,
            })
        } else {
            None
        };
        Ok(RecoveredRequest {
            request,
            dispatch: attempt.dispatch.clone(),
            completion_intent: attempt.completion_intent.clone(),
            preparation_recovery,
        })
    }

    fn consume_dispatch_permit(&self, permit: DispatchPermit) -> Result<DispatchedRequest> {
        if permit.owner_epoch != self.store.epoch
            || permit.root_id != self.store.root_id
            || permit.identity != self.config.identity
        {
            return Err(CampaignError::Conflict("stale dispatch owner epoch".into()));
        }
        let state = self.current(&permit.identity, &permit.dispatched_revision)?;
        let request = state
            .obligations
            .iter()
            .find(|item| item.obligation_id == permit.obligation_id && item.status == "executing")
            .and_then(|item| item.request.clone())
            .ok_or_else(|| CampaignError::Integrity("dispatched request absent".into()))?;
        let dispatch = state
            .progress
            .attempts
            .iter()
            .find(|item| item.preparation_operation_id == request.operation_id)
            .and_then(|item| item.dispatch.as_ref())
            .ok_or_else(|| CampaignError::Integrity("dispatch history absent".into()))?;
        if request.request_digest != permit.request_digest
            || dispatch.request_digest != permit.request_digest
            || dispatch.operation_id != permit.dispatch_operation_id
            || dispatch.command != permit.command
            || self
                .store
                .dispatch_slot(&permit.identity, &permit.obligation_id)?
                != Some((Some(permit.dispatch_operation_id), true))
        {
            return Err(CampaignError::Conflict(
                "dispatch permit differs from durable slot".into(),
            ));
        }
        Ok(DispatchedRequest {
            request,
            command: permit.command,
        })
    }

    fn replay_dispatch(
        &self,
        locator: &RecoveryLocator,
    ) -> Result<(DispatchRecord, OperationReceipt)> {
        if locator.kind != "authorize_dispatch" || locator.identity != self.config.identity {
            return Err(CampaignError::Contract(
                "dispatch locator kind or identity differs".into(),
            ));
        }
        let receipt = match self.store.reconcile(locator) {
            Reconcile::Committed(receipt) => receipt,
            Reconcile::Absent => return Err(CampaignError::Conflict("dispatch absent".into())),
            Reconcile::Conflicting(_) => {
                return Err(CampaignError::Conflict("dispatch content differs".into()));
            }
            Reconcile::Unresolved(reason) => return Err(CampaignError::Integrity(reason)),
        };
        let (_, state) = self
            .store
            .replay_result(&locator.operation_id)?
            .ok_or_else(|| CampaignError::Integrity("dispatch receipt absent".into()))?;
        let record = state
            .progress
            .attempts
            .iter()
            .filter_map(|attempt| attempt.dispatch.as_ref())
            .find(|record| record.operation_id == locator.operation_id && record.may_have_entered)
            .cloned()
            .ok_or_else(|| CampaignError::Integrity("dispatch history absent".into()))?;
        Ok((record, receipt))
    }

    /// Positive retry is internal until the execution owner supplies a real
    /// definite-pre-entry readback. It cannot be invoked with a serialized ref.
    #[allow(dead_code)] // Called by HP3 composition after custody integration.
    pub(crate) fn prepare_retry(
        &mut self,
        identity: &CampaignIdentity,
        expected: &CampaignRevision,
        obligation_id: &str,
        operation_id: &str,
        attempt_id: &str,
        evidence: CheckedPreEntryFailure,
    ) -> Result<Preparation> {
        self.check_identity(identity)?;
        evidence.validate()?;
        for (name, value) in [
            ("retry operation", operation_id),
            ("retry attempt", attempt_id),
            ("custody reference", evidence.custody_reference.as_str()),
        ] {
            require_text(value, name)?;
        }
        crate::require_sha(&evidence.custody_sha256, "pre-entry custody")?;
        let digest = campaign_digest(&json!({"identity":identity,"expectedRevision":expected,
            "obligationId":obligation_id,"operationId":operation_id,"attemptId":attempt_id,
            "custodyReference":evidence.custody_reference,"custodySha256":evidence.custody_sha256,
            "failedAttempt":evidence.attempt_id,"dispatchOperationId":evidence.dispatch_operation_id}))?;
        if let Some((receipt, state)) =
            self.replay(operation_id, "prepare_retry", &digest, identity)?
        {
            let request = state
                .obligations
                .iter()
                .find(|item| item.obligation_id == obligation_id)
                .and_then(|item| item.request.clone())
                .ok_or_else(|| CampaignError::Integrity("retry request absent".into()))?;
            return Ok(Preparation::Replayed {
                request: Box::new(request),
                receipt,
            });
        }
        let mut state = self.current(identity, expected)?;
        if state.phase != "review_ready" {
            return Ok(Preparation::NoEffect("retry phase differs".into()));
        }
        let obligation_index = state
            .obligations
            .iter()
            .position(|item| item.obligation_id == obligation_id && item.status == "executing")
            .ok_or_else(|| CampaignError::Conflict("active obligation absent".into()))?;
        let prior_request = state.obligations[obligation_index]
            .request
            .clone()
            .ok_or_else(|| CampaignError::Integrity("prior request absent".into()))?;
        for prior in &state.progress.attempts {
            if prior.attempt_id == attempt_id
                && self
                    .store
                    .slot_for_operation(identity, &prior.preparation_operation_id)?
                    .is_some_and(|slot| slot.obligation_id == obligation_id)
            {
                return Ok(Preparation::NoEffect(
                    "retry attempt identity differs".into(),
                ));
            }
        }
        let last = state
            .progress
            .attempts
            .iter_mut()
            .find(|item| item.preparation_operation_id == prior_request.operation_id)
            .ok_or_else(|| CampaignError::Integrity("prior attempt absent".into()))?;
        let dispatch = last
            .dispatch
            .as_ref()
            .ok_or_else(|| CampaignError::Conflict("prior dispatch absent".into()))?;
        if last.attempt_id != evidence.attempt_id
            || dispatch.operation_id != evidence.dispatch_operation_id
            || last.outcome.is_some()
            || last.completion_intent.is_some()
        {
            return Ok(Preparation::NoEffect(
                "prior attempt is not eligible for pre-entry retry".into(),
            ));
        }
        let prior_id = last.attempt_id.clone();
        last.outcome = Some(AttemptOutcome::DefinitePreEntryFailure);
        last.failure_custody = Some(FailureCustodyRef {
            reference: evidence.custody_reference.clone(),
            sha256: evidence.custody_sha256.clone(),
        });
        if prior_request.request_digest != last.request_digest
            || self.store.dispatch_slot(identity, obligation_id)?
                != Some((Some(dispatch.operation_id.clone()), true))
        {
            return Err(CampaignError::Integrity(
                "prior request or slot differs".into(),
            ));
        }
        let mut request = prior_request;
        request.operation_id = operation_id.into();
        request.kind = "retry".into();
        request.request_digest = campaign_digest(
            &json!({"rootId":request.root_id,"identity":request.identity,
            "obligationId":request.obligation_id,"operationId":operation_id,"kind":"retry",
            "attemptId":attempt_id,"predecessor":prior_id,"selection":request.selection,
            "candidate":request.candidate,"profileDigest":request.profile_digest}),
        )?;
        state.progress.attempts.push(AttemptRecord {
            attempt_id: attempt_id.into(),
            preparation_operation_id: operation_id.into(),
            request_digest: request.request_digest.clone(),
            predecessor: Some(prior_id),
            dispatch: None,
            completion_intent: None,
            outcome: None,
            failure_custody: None,
        });
        state.obligations[obligation_index].request = Some(request.clone());
        state.revision = state_revision(&state)?;
        request.prepared_revision = state.revision.clone();
        state.obligations[obligation_index].request = Some(request.clone());
        let receipt = self.receipt(
            &state,
            Some(expected.clone()),
            operation_id,
            "prepare_retry",
            digest.clone(),
        );
        let locator = self.locator(
            identity,
            operation_id,
            "prepare_retry",
            digest,
            Some(expected.clone()),
        );
        Ok(
            match self
                .store
                .write(&state, Some(expected), &receipt, false, Some(obligation_id))
            {
                Ok(()) => Preparation::Applied(Box::new(PreparedInitial {
                    request: request.clone(),
                    handle: AdmissionHandle {
                        owner_epoch: self.store.epoch.clone(),
                        root_id: self.store.root_id.clone(),
                        identity: identity.clone(),
                        obligation_id: obligation_id.into(),
                        operation_id: operation_id.into(),
                        request_digest: request.request_digest,
                        prepared_revision: state.revision.clone(),
                        expected_revision: state.revision,
                    },
                    receipt,
                })),
                Err(CampaignError::Conflict(reason)) | Err(CampaignError::Contract(reason)) => {
                    Preparation::NoEffect(reason)
                }
                Err(_) => Preparation::OutcomeUnknown(locator),
            },
        )
    }
}
