use std::fs;
#[cfg(feature = "test-faults")]
use std::path::PathBuf;
#[cfg(feature = "test-faults")]
use std::process::Command;

use claim_evidence::{AdmissionBinding, FindingAdmissionPort};
use serde_json::{Value, json};
use slice_campaign::contract::episode_digest;
#[cfg(feature = "test-faults")]
use slice_campaign::{AcceptedBoundary, Baseline};
use slice_campaign::{
    AdmitRequest, AdvancePhase, Campaign, CampaignIdentity, Consequence, DispatchCommand,
    DispatchEffect, Effect, Preparation, Reconcile,
};

mod support;
use support::*;

fn selected_campaign(
    specialist_count: usize,
    all_selected: bool,
) -> (
    tempfile::TempDir,
    Campaign,
    CampaignIdentity,
    slice_campaign::Snapshot,
) {
    let (temp, repo, workspace, identity, baseline, boundary) = setup();
    let root = temp.path().join("campaign");
    let mut app = Campaign::initialize(config(
        root,
        repo.clone(),
        workspace.clone(),
        identity.clone(),
        baseline.clone(),
        boundary.clone(),
    ))
    .unwrap();
    let accepted = applied(
        app.admit(
            "admit-many",
            AdmitRequest {
                identity: identity.clone(),
                repository: repo.clone(),
                workspace,
                accepted_boundary: boundary.clone(),
                baseline,
                expected_impact: None,
            },
        )
        .unwrap(),
    );
    let consequence = Consequence {
        owner: "slice-supervisor".into(),
        reference: "accepted-boundary".into(),
        sha256: boundary.sha256,
    };
    let implementing = applied(
        app.advance(
            &identity,
            &accepted.revision,
            "implement-many",
            AdvancePhase::Implementing,
            consequence.clone(),
        )
        .unwrap(),
    );
    let gate = applied(
        app.advance(
            &identity,
            &implementing.revision,
            "gate-many",
            AdvancePhase::GateReady,
            consequence.clone(),
        )
        .unwrap(),
    );
    let (candidate, profile) = anchored_artifacts(&repo);
    let bound = applied(
        app.bind_existing_candidate(
            &identity,
            &gate.revision,
            "candidate-many",
            &candidate,
            &profile,
        )
        .unwrap(),
    );
    let ready = applied(
        app.advance(
            &identity,
            &bound.revision,
            "ready-many",
            AdvancePhase::ReviewReady,
            consequence,
        )
        .unwrap(),
    );
    let c = ready.candidate.as_ref().unwrap();
    let subject = json!({"commit":c.commit,"tree":c.tree,"patchIdentity":c.patch_identity});
    let specialists: Vec<Value> = (0..specialist_count).map(|index| {
        let obligation = format!("review-{index}");
        if all_selected || index == 0 {
            json!({"obligationId":obligation,"skill":"implementation-review","selection":"selected",
                "requiredClaims":[claim(&identity,&obligation,&subject,"builder_projection"),
                    claim(&identity,&obligation,&subject,"campaign_terminalization")]})
        } else {
            json!({"obligationId":obligation,"skill":"implementation-review","selection":"omitted",
                "requiredClaims":[]})
        }
    }).collect();
    let selected = applied(
        app.bind_selection(
            &identity,
            &ready.revision,
            "selection-many",
            json!({"schemaVersion":2,"owner":"slice-supervisor","selectionId":"selection-many",
            "subject":subject,"specialists":specialists}),
        )
        .unwrap(),
    );
    (temp, app, identity, selected)
}

#[test]
fn independent_selected_obligations_prepare_dispatch_and_recover() {
    let (_temp, mut app, identity, selected) = selected_campaign(2, true);
    let first = match app
        .prepare_initial(&identity, &selected.revision, "review-0", "prepare-many-0")
        .unwrap()
    {
        Preparation::Applied(value) => value,
        _ => panic!("first preparation"),
    };
    let after_first = app.read(&identity).unwrap().unwrap();
    let second = match app
        .prepare_initial(
            &identity,
            &after_first.revision,
            "review-1",
            "prepare-many-1",
        )
        .unwrap()
    {
        Preparation::Applied(value) => value,
        _ => panic!("second preparation"),
    };
    assert_eq!(
        app.read(&identity)
            .unwrap()
            .unwrap()
            .progress
            .attempts
            .len(),
        2
    );
    for (obligation, prepared, operation) in [
        ("review-0", first, "dispatch-many-0"),
        ("review-1", second, "dispatch-many-1"),
    ] {
        let locator = slice_campaign::RecoveryLocator {
            root_id: app.root_id().into(),
            anchored_root: app.anchored_root().into(),
            identity: identity.clone(),
            operation_id: prepared.request.operation_id.clone(),
            kind: "prepare_initial".into(),
            request_digest: prepared.receipt.request_digest.clone(),
            expected_revision: prepared.receipt.prior_revision.clone(),
            profile_digest: prepared.receipt.profile_digest.clone(),
        };
        let recovered = app.recover_request(&locator, obligation).unwrap();
        let handle = recovered
            .preparation_recovery
            .expect("independent recovery handle");
        let episode =
            episode_digest(&json!({"identity":identity,"obligationId":obligation})).unwrap();
        let command = DispatchCommand {
            episode_id: episode[..32].into(),
            writer_actor: "native-review-host".into(),
            runtime_session: format!("session-{obligation}"),
            reviewer_profile: "direct-initial".into(),
            begin_transition_id: format!("begin-{obligation}"),
            result_transition_id: format!("result-{obligation}"),
        };
        let DispatchEffect::Applied { permit, .. } = app
            .authorize_recovered_dispatch(handle, operation, command)
            .unwrap()
        else {
            panic!("independent dispatch")
        };
        app.consume_dispatch_permit(permit).unwrap();
        assert!(
            app.recover_request(&locator, obligation)
                .unwrap()
                .dispatch
                .is_some()
        );
        assert!(matches!(
            app.prepare_initial(
                &identity,
                prepared.receipt.prior_revision.as_ref().unwrap(),
                obligation,
                &prepared.request.operation_id
            )
            .unwrap(),
            Preparation::Replayed { .. }
        ));
    }
}

#[test]
fn accepted_129_specialist_selection_remains_readable() {
    let (_temp, app, identity, selected) = selected_campaign(129, false);
    assert_eq!(selected.obligations.len(), 129);
    assert_eq!(app.read(&identity).unwrap().unwrap().obligations.len(), 129);
}

#[test]
fn full_initial_chain_reopens_without_regranting_entry() {
    let (temp, repo, workspace, identity, baseline, boundary) = setup();
    let root = temp.path().join("campaign");
    let mut app = Campaign::initialize(config(
        root.clone(),
        repo.clone(),
        workspace.clone(),
        identity.clone(),
        baseline.clone(),
        boundary.clone(),
    ))
    .unwrap();
    let request = AdmitRequest {
        identity: identity.clone(),
        repository: repo.clone(),
        workspace: workspace.clone(),
        accepted_boundary: boundary.clone(),
        baseline: baseline.clone(),
        expected_impact: None,
    };
    let accepted = applied(app.admit("admit-1", request).unwrap());
    assert_eq!(accepted.phase, "accepted");
    let consequence = Consequence {
        owner: "slice-supervisor".into(),
        reference: "accepted-boundary".into(),
        sha256: boundary.sha256.clone(),
    };
    let implementing = applied(
        app.advance(
            &identity,
            &accepted.revision,
            "advance-1",
            AdvancePhase::Implementing,
            consequence.clone(),
        )
        .unwrap(),
    );
    let gate = applied(
        app.advance(
            &identity,
            &implementing.revision,
            "advance-2",
            AdvancePhase::GateReady,
            consequence.clone(),
        )
        .unwrap(),
    );
    let replay = app
        .advance(
            &identity,
            &accepted.revision,
            "advance-1",
            AdvancePhase::Implementing,
            Consequence {
                owner: "slice-supervisor".into(),
                reference: "accepted-boundary".into(),
                sha256: boundary.sha256.clone(),
            },
        )
        .unwrap();
    match replay {
        Effect::Replayed { value, receipt } => {
            assert_eq!(value.phase, "implementing");
            assert_eq!(receipt.result_revision, implementing.revision);
        }
        _ => panic!("exact old operation must replay"),
    }
    let (candidate, profile) = anchored_artifacts(&repo);
    let bound = applied(
        app.bind_existing_candidate(
            &identity,
            &gate.revision,
            "bind-candidate-1",
            &candidate,
            &profile,
        )
        .unwrap(),
    );
    let ready = applied(
        app.advance(
            &identity,
            &bound.revision,
            "advance-3",
            AdvancePhase::ReviewReady,
            consequence,
        )
        .unwrap(),
    );
    let candidate_ref = ready.candidate.clone().unwrap();
    let subject = json!({"commit":candidate_ref.commit,"tree":candidate_ref.tree,"patchIdentity":candidate_ref.patch_identity});
    let selection = json!({"schemaVersion":2,"owner":"slice-supervisor","selectionId":"selection-1","subject":subject,"specialists":[{"obligationId":"review-1","skill":"implementation-review","selection":"selected","requiredClaims":[claim(&identity,"review-1",&subject,"builder_projection"),claim(&identity,"review-1",&subject,"campaign_terminalization")]},{"obligationId":"review-2","skill":"implementation-review","selection":"omitted","requiredClaims":[]}]});
    let selected = applied(
        app.bind_selection(&identity, &ready.revision, "selection-1", selection)
            .unwrap(),
    );
    let prepared = app
        .prepare_initial(&identity, &selected.revision, "review-1", "prepare-1")
        .unwrap();
    let (request, handle, receipt) = match prepared {
        Preparation::Applied(value) => (value.request, value.handle, value.receipt),
        _ => panic!("not applied"),
    };
    assert_eq!(request.kind, "initial");
    assert_eq!(
        app.read_prepared(&identity, "review-1")
            .unwrap()
            .unwrap()
            .request_digest,
        request.request_digest
    );
    let replay = app
        .prepare_initial(&identity, &selected.revision, "review-1", "prepare-1")
        .unwrap();
    assert!(matches!(replay, Preparation::Replayed { .. }));
    let locator = slice_campaign::RecoveryLocator {
        root_id: app.root_id().into(),
        anchored_root: root.clone(),
        identity: identity.clone(),
        operation_id: "prepare-1".into(),
        kind: "prepare_initial".into(),
        request_digest: receipt.request_digest.clone(),
        expected_revision: Some(selected.revision),
        profile_digest: receipt.profile_digest.clone(),
    };
    assert!(matches!(
        app.reconcile_operation(&locator),
        Reconcile::Committed(_)
    ));
    drop(app);
    let mut reopened = Campaign::open(config(
        root.clone(),
        repo.clone(),
        workspace.clone(),
        identity.clone(),
        baseline.clone(),
        boundary.clone(),
    ))
    .unwrap();
    assert!(reopened.consume_initial_admission(handle).is_err());
    assert_eq!(
        reopened
            .read_prepared(&identity, "review-1")
            .unwrap()
            .unwrap()
            .request_digest,
        request.request_digest
    );
    assert!(matches!(
        reopened.reconcile_operation(&locator),
        Reconcile::Committed(_)
    ));
    let recovered = reopened.recover_request(&locator, "review-1").unwrap();
    assert!(recovered.dispatch.is_none());
    let recovered_handle = recovered
        .preparation_recovery
        .expect("unentered preparation can be re-admitted");
    let episode_id =
        episode_digest(&json!({"identity":identity,"obligationId":"review-1"})).unwrap();
    let command = DispatchCommand {
        episode_id: episode_id[..32].into(),
        writer_actor: "native-review-host".into(),
        runtime_session: "session-1".into(),
        reviewer_profile: "direct-initial".into(),
        begin_transition_id: "begin-1".into(),
        result_transition_id: "result-1".into(),
    };
    let effect = reopened
        .authorize_recovered_dispatch(recovered_handle, "dispatch-1", command.clone())
        .unwrap();
    let (permit, dispatch_receipt) = match effect {
        DispatchEffect::Applied { permit, receipt } => (permit, receipt),
        _ => panic!("expected one committed dispatch"),
    };
    let dispatch_locator = slice_campaign::RecoveryLocator {
        root_id: reopened.root_id().into(),
        anchored_root: reopened.anchored_root().into(),
        identity: identity.clone(),
        operation_id: "dispatch-1".into(),
        kind: "authorize_dispatch".into(),
        request_digest: dispatch_receipt.request_digest.clone(),
        expected_revision: dispatch_receipt.prior_revision.clone(),
        profile_digest: dispatch_receipt.profile_digest.clone(),
    };
    let dispatched = reopened.consume_dispatch_permit(permit).unwrap();
    assert_eq!(dispatched.command, command);
    assert_eq!(dispatched.request.request_digest, request.request_digest);
    let (replayed, _) = reopened.replay_dispatch(&dispatch_locator).unwrap();
    assert!(replayed.may_have_entered);
    let mut conflicting = dispatch_locator.clone();
    conflicting.request_digest = "0".repeat(64);
    assert!(reopened.replay_dispatch(&conflicting).is_err());
    let current = reopened.read(&identity).unwrap().unwrap();
    let binding = AdmissionBinding {
        campaign_root_id: reopened.root_id().into(),
        campaign_revision: current.revision.as_str().into(),
        campaign_operation_id: "unregistered-intent".into(),
        obligation_id: "review-1".into(),
        candidate_digest: request.candidate.receipt_sha256.clone(),
        selection_digest: request.selection.campaign_digest.clone(),
        profile_digest: request.profile_digest.clone(),
        prepared_request_sha256: request.request_digest.clone(),
        episode_revision: "episode-revision".into(),
        result_sha256: "1".repeat(64),
        claims_request_sha256: "2".repeat(64),
    };
    assert!(reopened.claims_admission().acquire(&binding).is_err());
    let recovered = reopened.recover_request(&locator, "review-1").unwrap();
    assert!(recovered.preparation_recovery.is_none());
    assert!(recovered.dispatch.unwrap().may_have_entered);
    drop(reopened);
    let reopened =
        Campaign::open(config(root, repo, workspace, identity, baseline, boundary)).unwrap();
    assert!(
        reopened
            .recover_request(&locator, "review-1")
            .unwrap()
            .preparation_recovery
            .is_none()
    );
}

#[test]
fn fresh_initial_handle_consumes_exact_durable_request() {
    let (temp, repo, workspace, identity, baseline, boundary) = setup();
    let mut app = Campaign::initialize(config(
        temp.path().join("campaign"),
        repo.clone(),
        workspace.clone(),
        identity.clone(),
        baseline.clone(),
        boundary.clone(),
    ))
    .unwrap();
    let accepted = applied(
        app.admit(
            "admit-fresh",
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
    let implementing = applied(
        app.advance(
            &identity,
            &accepted.revision,
            "implement-fresh",
            AdvancePhase::Implementing,
            consequence.clone(),
        )
        .unwrap(),
    );
    let gate = applied(
        app.advance(
            &identity,
            &implementing.revision,
            "gate-fresh",
            AdvancePhase::GateReady,
            consequence.clone(),
        )
        .unwrap(),
    );
    let (candidate, profile) = anchored_artifacts(&repo);
    let bound = applied(
        app.bind_existing_candidate(
            &identity,
            &gate.revision,
            "candidate-fresh",
            &candidate,
            &profile,
        )
        .unwrap(),
    );
    let ready = applied(
        app.advance(
            &identity,
            &bound.revision,
            "ready-fresh",
            AdvancePhase::ReviewReady,
            consequence,
        )
        .unwrap(),
    );
    let candidate_ref = ready.candidate.clone().unwrap();
    let subject = json!({"commit":candidate_ref.commit,"tree":candidate_ref.tree,"patchIdentity":candidate_ref.patch_identity});
    let selection = json!({"schemaVersion":2,"owner":"slice-supervisor","selectionId":"selection-fresh","subject":subject,"specialists":[{"obligationId":"review-1","skill":"implementation-review","selection":"selected","requiredClaims":[claim(&identity,"review-1",&subject,"builder_projection"),claim(&identity,"review-1",&subject,"campaign_terminalization")]}]});
    let selected = applied(
        app.bind_selection(&identity, &ready.revision, "selection-fresh", selection)
            .unwrap(),
    );
    let prepared = app
        .prepare_initial(&identity, &selected.revision, "review-1", "prepare-fresh")
        .unwrap();
    let Preparation::Applied(prepared) = prepared else {
        panic!("expected fresh preparation")
    };
    let expected = prepared.request.clone();
    let consumed = app.consume_initial_admission(prepared.handle).unwrap();
    assert_eq!(consumed, expected);
    assert_eq!(
        app.read_prepared(&identity, "review-1").unwrap(),
        Some(expected.clone())
    );
    assert!(
        matches!(app.prepare_initial(&identity, &selected.revision, "review-1", "prepare-fresh").unwrap(), Preparation::Replayed { request, .. } if *request == expected)
    );
}

#[test]
fn historical_schema_marker_is_not_upgraded_on_open() {
    let (temp, repo, workspace, identity, baseline, boundary) = setup();
    let root = temp.path().join("campaign");
    let app = Campaign::initialize(config(
        root.clone(),
        repo.clone(),
        workspace.clone(),
        identity.clone(),
        baseline.clone(),
        boundary.clone(),
    ))
    .unwrap();
    drop(app);
    let current = root.join(".campaign-native-initial-v2");
    let historical = root.join(".campaign-native-initial-v1");
    let mut marker: Value = serde_json::from_slice(&fs::read(&current).unwrap()).unwrap();
    marker["schema_version"] = json!(1);
    fs::write(&historical, serde_json::to_vec(&marker).unwrap()).unwrap();
    fs::remove_file(current).unwrap();
    assert!(Campaign::open(config(root, repo, workspace, identity, baseline, boundary)).is_err());
}

#[test]
fn substituted_database_cannot_report_committed_operation_absent() {
    let (temp, repo, workspace, identity, baseline, boundary) = setup();
    let root_a = temp.path().join("campaign-a");
    let root_b = temp.path().join("campaign-b");
    let mut app = Campaign::initialize(config(
        root_a.clone(),
        repo.clone(),
        workspace.clone(),
        identity.clone(),
        baseline.clone(),
        boundary.clone(),
    ))
    .unwrap();
    let admitted = app
        .admit(
            "admit-for-substitution",
            AdmitRequest {
                identity: identity.clone(),
                repository: repo.clone(),
                workspace: workspace.clone(),
                accepted_boundary: boundary.clone(),
                baseline: baseline.clone(),
                expected_impact: None,
            },
        )
        .unwrap();
    let receipt = match admitted {
        Effect::Applied { receipt, .. } => receipt,
        _ => panic!("admission not applied"),
    };
    let locator = slice_campaign::RecoveryLocator {
        root_id: app.root_id().into(),
        anchored_root: root_a.clone(),
        identity: identity.clone(),
        operation_id: "admit-for-substitution".into(),
        kind: "admit".into(),
        request_digest: receipt.request_digest,
        expected_revision: None,
        profile_digest: receipt.profile_digest,
    };
    assert!(matches!(
        app.reconcile_operation(&locator),
        Reconcile::Committed(_)
    ));
    let other = Campaign::initialize(config(
        root_b.clone(),
        repo.clone(),
        workspace.clone(),
        identity.clone(),
        baseline.clone(),
        boundary.clone(),
    ))
    .unwrap();
    drop(other);
    fs::rename(
        root_b.join("slice-campaign.sqlite"),
        root_a.join("slice-campaign.sqlite"),
    )
    .unwrap();
    assert!(app.read(&identity).is_err());
    assert!(matches!(
        app.reconcile_operation(&locator),
        Reconcile::Unresolved(_)
    ));
    drop(app);
    assert!(
        Campaign::open(config(
            root_a, repo, workspace, identity, baseline, boundary
        ))
        .is_err()
    );
}

#[test]
fn private_root_refuses_other_writer_and_symlinks() {
    use std::os::unix::fs::symlink;
    let (temp, repo, workspace, identity, baseline, boundary) = setup();
    let root = temp.path().join("campaign");
    let app = Campaign::initialize(config(
        root.clone(),
        repo.clone(),
        workspace.clone(),
        identity.clone(),
        baseline.clone(),
        boundary.clone(),
    ))
    .unwrap();
    assert!(
        Campaign::open(config(
            root.clone(),
            repo.clone(),
            workspace.clone(),
            identity.clone(),
            baseline.clone(),
            boundary.clone()
        ))
        .is_err()
    );
    let alias = temp.path().join("alias");
    symlink(&root, &alias).unwrap();
    assert!(
        Campaign::open(config(
            alias,
            repo.clone(),
            workspace.clone(),
            identity.clone(),
            baseline.clone(),
            boundary.clone()
        ))
        .is_err()
    );
    drop(app);
    let sidecar = root.join("slice-campaign.sqlite-wal");
    if sidecar.exists() {
        fs::remove_file(&sidecar).unwrap();
    }
    symlink(temp.path().join("outside"), &sidecar).unwrap();
    assert!(Campaign::open(config(root, repo, workspace, identity, baseline, boundary)).is_err());
}

#[test]
fn selected_claim_pair_keeps_distinct_boundaries_and_codec_identity() {
    use slice_campaign::contract::{CandidateRef, validate_selection};
    let identity = CampaignIdentity {
        run_id: "sc0-oracle-c2".into(),
        slice_number: 1,
        attempt_id: "sc0-attempt".into(),
        plan_version: "sc0-controlled-v1".into(),
    };
    let candidate = CandidateRef {
        commit: "49624730af1663f50a3ff85abfdd1b90647d0cdb".into(),
        tree: "882023b5f0d6b89425f151aab85e6534a53cf5bb".into(),
        patch_identity: "3fc3693325147d6f2c839116f9b2b793b7ce5e4d3182aa4c747f2d55ae6c099e".into(),
        receipt_sha256: "4a2c4f1872f6dfe8de275181f08b1e4d8c33a437c2519cd8b9ce8b3c9b56a5cd".into(),
        physical_profile_sha256: "5f78af676bf50a7a328b12e0ddcbf619ecfd78fd8e2f46f299330ff0d96095db"
            .into(),
    };
    let subject = json!({"commit":candidate.commit,"tree":candidate.tree,"patchIdentity":candidate.patch_identity});
    let mut selection = json!({"schemaVersion":2,"owner":"slice-supervisor","selectionId":"selection-1","subject":subject,"specialists":[{"obligationId":"review-1","skill":"implementation-review","selection":"selected","requiredClaims":[claim(&identity,"review-1",&subject,"builder_projection"),claim(&identity,"review-1",&subject,"campaign_terminalization")]}]});
    let reference = validate_selection(&selection, &identity, &candidate).unwrap();
    assert_eq!(
        reference.campaign_digest,
        slice_campaign::codec::campaign_digest(&selection).unwrap()
    );
    assert_eq!(
        reference.episode_authority_digest,
        episode_digest(&selection).unwrap()
    );
    selection["specialists"][0]["requiredClaims"][1]["claimId"] =
        json!("production-path-claim-v1@".to_owned() + &"0".repeat(64));
    assert!(validate_selection(&selection, &identity, &candidate).is_err());
    selection["specialists"][0]["requiredClaims"][1] =
        selection["specialists"][0]["requiredClaims"][0].clone();
    assert!(validate_selection(&selection, &identity, &candidate).is_err());
}

#[cfg(feature = "test-faults")]
#[test]
fn fault_helper_admit() {
    let Ok(root) = std::env::var("SC1_HELPER_ROOT") else {
        return;
    };
    let repo = PathBuf::from(std::env::var("SC1_HELPER_REPO").unwrap());
    let workspace = PathBuf::from(std::env::var("SC1_HELPER_WORKSPACE").unwrap());
    let identity = CampaignIdentity {
        run_id: "sc0-oracle-c2".into(),
        slice_number: 1,
        attempt_id: "sc0-attempt".into(),
        plan_version: "sc0-controlled-v1".into(),
    };
    let baseline = Baseline {
        accepted_commit: "a8e074d765d94a7e816b7ab49db428fd4a760a9f".into(),
        accepted_tree: "17868afbf46ea53715981fce83171001c632b957".into(),
        inter_slice_commit: "a8e074d765d94a7e816b7ab49db428fd4a760a9f".into(),
    };
    let boundary = AcceptedBoundary {
        reference:
            "controlled SC0 candidate-2 frozen source-tree gate fixture under accepted HP2 SC0 plan"
                .into(),
        sha256: "f45b94d3ef985a446002e21a795a51004a5c732e5b150208be09f964152662ef".into(),
    };
    let mut app = Campaign::initialize(config(
        PathBuf::from(root),
        repo.clone(),
        workspace.clone(),
        identity.clone(),
        baseline.clone(),
        boundary.clone(),
    ))
    .unwrap();
    let request = AdmitRequest {
        identity,
        repository: repo,
        workspace,
        accepted_boundary: boundary,
        baseline,
        expected_impact: None,
    };
    let _ = app.admit("fault-admit", request);
}

#[cfg(feature = "test-faults")]
#[test]
fn killed_before_and_after_sqlite_commit_reconcile_exactly() {
    use slice_campaign::RecoveryLocator;
    use slice_campaign::codec::campaign_digest;
    use std::time::{Duration, Instant};
    for cut in ["before_commit", "after_commit"] {
        let (temp, repo, workspace, identity, baseline, boundary) = setup();
        let root = temp.path().join("campaign");
        let signals = temp.path().join("signals");
        fs::create_dir(&signals).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "fault_helper_admit", "--nocapture"])
            .env("SC1_HELPER_ROOT", &root)
            .env("SC1_HELPER_REPO", &repo)
            .env("SC1_HELPER_WORKSPACE", &workspace)
            .env("SLICE_CAMPAIGN_FAULT_CUT", cut)
            .env("SLICE_CAMPAIGN_FAULT_DIR", &signals)
            .spawn()
            .unwrap();
        let start = Instant::now();
        while !signals.join(format!("{cut}.ready")).exists() {
            if start.elapsed() > Duration::from_secs(5) {
                child.kill().unwrap();
                panic!("fault cut did not arrive");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        child.kill().unwrap();
        child.wait().unwrap();
        let app = Campaign::open(config(
            root.clone(),
            repo.clone(),
            workspace.clone(),
            identity.clone(),
            baseline.clone(),
            boundary.clone(),
        ))
        .unwrap();
        let marker: Value =
            serde_json::from_slice(&fs::read(root.join(".campaign-native-initial-v2")).unwrap())
                .unwrap();
        let digest=campaign_digest(&json!({"operationId":"fault-admit","identity":identity,"repository":repo,"workspace":workspace,"acceptedBoundary":boundary,"baseline":baseline,"expectedImpact":null})).unwrap();
        let locator = RecoveryLocator {
            root_id: app.root_id().into(),
            anchored_root: root,
            identity,
            operation_id: "fault-admit".into(),
            kind: "admit".into(),
            request_digest: digest,
            expected_revision: None,
            profile_digest: marker["trusted_config_digest"].as_str().unwrap().into(),
        };
        match cut {
            "before_commit" => assert!(matches!(
                app.reconcile_operation(&locator),
                Reconcile::Absent
            )),
            "after_commit" => assert!(matches!(
                app.reconcile_operation(&locator),
                Reconcile::Committed(_)
            )),
            _ => unreachable!(),
        }
    }
}
