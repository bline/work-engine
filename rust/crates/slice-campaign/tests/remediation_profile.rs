#![cfg(unix)]
mod support;

use serde_json::json;
use slice_campaign::{
    AcceptedBoundary, AdmitRequest, AdvancePhase, Campaign, Consequence, Preparation, TrustedConfig,
};

fn v4_config(
    root: std::path::PathBuf,
    repo: std::path::PathBuf,
    workspace: std::path::PathBuf,
    identity: slice_campaign::CampaignIdentity,
    baseline: slice_campaign::Baseline,
    boundary: AcceptedBoundary,
) -> TrustedConfig {
    TrustedConfig::remediation_v4(
        root,
        repo,
        workspace,
        identity,
        boundary,
        baseline,
        "sc3-fixture-writer".into(),
        support::fixture("f52792e22d2a1b73e007bc7078607d6657989248a35de1371a1180d23101d206"),
        support::fixture("d155cc2ac1cf1ab26b0e186d6a6ac86f547b1a0b9c3ce74f0ba69eecd37a2f34"),
        support::fixture("c927b51b887e37f6fdc02e1fabfadddd211d0ac30712cbc6c5028993ad3889c2"),
    )
    .unwrap()
}

#[test]
fn fresh_schema_four_runs_initial_admission_and_refuses_cross_profile_open() {
    let (temp, repo, workspace, identity, baseline, _) = support::setup();
    let boundary = AcceptedBoundary {
        reference: "accepted fixture campaign".into(),
        sha256: "4".repeat(64),
    };
    let root = temp.path().join("campaign-v4");
    let mut app = Campaign::initialize(v4_config(
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
            "admit-v4",
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
    assert_eq!(accepted.schema_version, 4);
    let consequence = Consequence {
        owner: "slice-supervisor".into(),
        reference: "accepted fixture campaign".into(),
        sha256: boundary.sha256.clone(),
    };
    let implementing = support::applied(
        app.advance(
            &identity,
            &accepted.revision,
            "implement-v4",
            AdvancePhase::Implementing,
            consequence.clone(),
        )
        .unwrap(),
    );
    let gated = support::applied(
        app.advance(
            &identity,
            &implementing.revision,
            "gate-v4",
            AdvancePhase::GateReady,
            consequence.clone(),
        )
        .unwrap(),
    );
    let (candidate, profile) = support::anchored_artifacts(&repo);
    let bound = support::applied(
        app.bind_existing_candidate(
            &identity,
            &gated.revision,
            "candidate-v4",
            &candidate,
            &profile,
        )
        .unwrap(),
    );
    let ready = support::applied(
        app.advance(
            &identity,
            &bound.revision,
            "ready-v4",
            AdvancePhase::ReviewReady,
            consequence,
        )
        .unwrap(),
    );
    let subject = json!({"commit":ready.candidate.as_ref().unwrap().commit,
        "tree":ready.candidate.as_ref().unwrap().tree,
        "patchIdentity":ready.candidate.as_ref().unwrap().patch_identity});
    let obligation = "implementation-review";
    let selected = support::applied(
        app.bind_selection(
            &identity,
            &ready.revision,
            "select-v4",
            json!({"schemaVersion":2,"owner":"slice-supervisor","selectionId":"selected-v4",
            "subject":subject,"specialists":[{"obligationId":obligation,
            "skill":"implementation-review","selection":"selected","requiredClaims":[
                support::claim(&identity, obligation, &subject, "builder_projection"),
                support::claim(&identity, obligation, &subject, "campaign_terminalization")]}]}),
        )
        .unwrap(),
    );
    let preparation = app
        .prepare_initial(&identity, &selected.revision, obligation, "prepare-v4")
        .unwrap();
    assert!(matches!(preparation, Preparation::Applied(_)));
    drop(app);
    assert!(Campaign::open(support::config(root.clone(), repo.clone(), workspace.clone(),
        identity.clone(), baseline.clone(), AcceptedBoundary {
            reference: "controlled SC0 candidate-2 frozen source-tree gate fixture under accepted HP2 SC0 plan".into(),
            sha256: "f45b94d3ef985a446002e21a795a51004a5c732e5b150208be09f964152662ef".into(),
        })).is_err());
    let reopened = Campaign::open(v4_config(
        root,
        repo,
        workspace,
        identity.clone(),
        baseline,
        boundary,
    ))
    .unwrap();
    let current = reopened.read(&identity).unwrap().unwrap();
    assert_eq!(current.schema_version, 4);
    assert_eq!(current.progress.attempts.len(), 1);
    assert!(matches!(
        reopened.reconcile_operation(&slice_campaign::RecoveryLocator {
            root_id: reopened.root_id().into(),
            anchored_root: reopened.anchored_root().into(),
            identity,
            operation_id: "prepare-v4".into(),
            kind: "prepare_initial".into(),
            request_digest: "0".repeat(64),
            expected_revision: None,
            profile_digest: "0".repeat(64),
        }),
        slice_campaign::Reconcile::Conflicting(_)
    ));
}
