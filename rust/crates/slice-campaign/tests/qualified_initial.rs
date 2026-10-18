#![cfg(feature = "test-support")]
#[path = "support/mod.rs"]
mod support;

use serde_json::json;
use slice_campaign::{
    AcceptedExecutionSelection, AdmitRequest, AdvancePhase, Campaign, Consequence, DispatchCommand,
    DispatchEffect, OfflineAcceptancePin, Preparation, TrustedConfig,
};

#[test]
fn offline_acceptance_uses_the_fresh_applied_owner_slot_once() {
    let (temp, repo, workspace, identity, baseline, boundary) = support::setup();
    let gate = [
        support::fixture("f52792e22d2a1b73e007bc7078607d6657989248a35de1371a1180d23101d206"),
        support::fixture("d155cc2ac1cf1ab26b0e186d6a6ac86f547b1a0b9c3ce74f0ba69eecd37a2f34"),
        support::fixture("c927b51b887e37f6fdc02e1fabfadddd211d0ac30712cbc6c5028993ad3889c2"),
    ];
    let (candidate, profile) = support::anchored_artifacts(&repo);
    let checked = slice_campaign::verify_existing_subject(
        &repo, &identity, &baseline, &candidate, &profile, &gate[0], &gate[1], &gate[2],
    )
    .unwrap();
    let subject = json!({"commit":checked.reference.commit,"tree":checked.reference.tree,
        "patchIdentity":checked.reference.patch_identity});
    let obligation = "review-1";
    let builder = support::claim(&identity, obligation, &subject, "builder_projection");
    let campaign_claim =
        support::claim(&identity, obligation, &subject, "campaign_terminalization");
    let selection = json!({"schemaVersion":2,"owner":"slice-supervisor","selectionId":"selection-rp2",
        "subject":subject,"specialists":[{"obligationId":obligation,"skill":"implementation-review",
        "selection":"selected","requiredClaims":[builder,campaign_claim]}]});
    let selected =
        slice_campaign::contract::validate_selection(&selection, &identity, &checked.reference)
            .unwrap();
    let accepted = json!({
        "schemaVersion":1,"decision":"accepted","reference":"offline-rp2-test","revision":"revision-1",
        "identity":identity,"obligationId":obligation,"candidateTree":checked.reference.tree,
        "builderClaimId":selection["specialists"][0]["requiredClaims"][0]["claimId"],
        "campaignClaimId":selection["specialists"][0]["requiredClaims"][1]["claimId"],
        "selectionDigest":selected.campaign_digest,"physicalProfileDigest":checked.profile_digest,
        "claudeProfileDigest":"a".repeat(64),"programSha256":"b".repeat(64),
        "guardSha256":"c".repeat(64),"cbmStoreSha256":"d".repeat(64),
        "catalogSha256":"e".repeat(64),"requestedModel":"offline-model",
        "credentialSourceMetadataSha256":"f".repeat(64),
        "resourcePolicy":"strict_aggregate_session_64mib_v1",
        "sessionId":"session-rp2","attemptId":identity.attempt_id,
        "acceptedBoundarySha256":boundary.sha256,
        "promptTemplateSha256":"9".repeat(64),"attemptHorizonMillis":60_000,
    });
    let accepted_raw = serde_json::to_vec(&accepted).unwrap();
    let pin = OfflineAcceptancePin::test_owned(&accepted_raw).unwrap();
    let selected_execution = AcceptedExecutionSelection {
        claude_profile_digest: accepted["claudeProfileDigest"].as_str().unwrap(),
        program_sha256: accepted["programSha256"].as_str().unwrap(),
        guard_sha256: accepted["guardSha256"].as_str().unwrap(),
        cbm_store_sha256: accepted["cbmStoreSha256"].as_str().unwrap(),
        catalog_sha256: accepted["catalogSha256"].as_str().unwrap(),
        requested_model: accepted["requestedModel"].as_str().unwrap(),
        credential_source_metadata_sha256: accepted["credentialSourceMetadataSha256"]
            .as_str()
            .unwrap(),
        session_id: accepted["sessionId"].as_str().unwrap(),
        attempt_id: accepted["attemptId"].as_str().unwrap(),
        resource_policy: accepted["resourcePolicy"].as_str().unwrap(),
        prompt_template_sha256: accepted["promptTemplateSha256"].as_str().unwrap(),
        attempt_horizon_millis: 60_000,
    };
    pin.validate_execution(&selected_execution).unwrap();
    let mut accepted_v2 = accepted.clone();
    accepted_v2["resourcePolicy"] = json!("session64mib_combined1g_zero_swap_v2");
    let v2_pin =
        OfflineAcceptancePin::test_owned(&serde_json::to_vec(&accepted_v2).unwrap()).unwrap();
    let v2_selection = AcceptedExecutionSelection {
        claude_profile_digest: selected_execution.claude_profile_digest,
        program_sha256: selected_execution.program_sha256,
        guard_sha256: selected_execution.guard_sha256,
        cbm_store_sha256: selected_execution.cbm_store_sha256,
        catalog_sha256: selected_execution.catalog_sha256,
        requested_model: selected_execution.requested_model,
        credential_source_metadata_sha256: selected_execution.credential_source_metadata_sha256,
        session_id: selected_execution.session_id,
        attempt_id: selected_execution.attempt_id,
        resource_policy: accepted_v2["resourcePolicy"].as_str().unwrap(),
        prompt_template_sha256: selected_execution.prompt_template_sha256,
        attempt_horizon_millis: selected_execution.attempt_horizon_millis,
    };
    v2_pin.validate_execution(&v2_selection).unwrap();
    assert!(v2_pin.validate_execution(&selected_execution).is_err());
    assert!(pin.validate_execution(&v2_selection).is_err());
    assert!(
        pin.validate_execution(&AcceptedExecutionSelection {
            requested_model: "changed-model",
            ..selected_execution
        })
        .is_err()
    );
    let config = || {
        TrustedConfig::qualified_initial_offline_v4(
            temp.path().join("campaign"),
            repo.clone(),
            workspace.clone(),
            identity.clone(),
            boundary.clone(),
            baseline.clone(),
            "rp2-offline-writer".into(),
            gate[0].clone(),
            gate[1].clone(),
            gate[2].clone(),
            &pin,
        )
        .unwrap()
    };
    let mut app = Campaign::initialize(config()).unwrap();
    let admitted = support::applied(
        app.admit(
            "admit-rp2",
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
        reference: "accepted".into(),
        sha256: boundary.sha256.clone(),
    };
    let implementing = support::applied(
        app.advance(
            &identity,
            &admitted.revision,
            "implement-rp2",
            AdvancePhase::Implementing,
            consequence.clone(),
        )
        .unwrap(),
    );
    let gate_ready = support::applied(
        app.advance(
            &identity,
            &implementing.revision,
            "gate-rp2",
            AdvancePhase::GateReady,
            consequence.clone(),
        )
        .unwrap(),
    );
    let bound = support::applied(
        app.bind_existing_candidate(
            &identity,
            &gate_ready.revision,
            "candidate-rp2",
            &candidate,
            &profile,
        )
        .unwrap(),
    );
    let ready = support::applied(
        app.advance(
            &identity,
            &bound.revision,
            "ready-rp2",
            AdvancePhase::ReviewReady,
            consequence,
        )
        .unwrap(),
    );
    let selected_state = support::applied(
        app.bind_selection(&identity, &ready.revision, "selection-rp2", selection)
            .unwrap(),
    );
    let mut changed = accepted.clone();
    changed["requestedModel"] = json!("changed-model");
    assert!(
        app.install_offline_initial_authority(
            OfflineAcceptancePin::test_owned(&serde_json::to_vec(&changed).unwrap()).unwrap()
        )
        .is_err()
    );
    app.install_offline_initial_authority(OfflineAcceptancePin::test_owned(&accepted_raw).unwrap())
        .unwrap();
    assert!(
        app.install_offline_initial_authority(
            OfflineAcceptancePin::test_owned(&accepted_raw).unwrap()
        )
        .is_err()
    );
    let Preparation::Applied(prepared) = app
        .prepare_initial(
            &identity,
            &selected_state.revision,
            obligation,
            "prepare-rp2",
        )
        .unwrap()
    else {
        panic!("fresh preparation expected")
    };
    let command = DispatchCommand {
        episode_id: slice_campaign::contract::episode_digest(&json!({"identity":identity,
            "obligationId":obligation}))
        .unwrap()[..32]
            .into(),
        writer_actor: "native-review-host".into(),
        runtime_session: "session-rp2".into(),
        reviewer_profile: "direct-initial".into(),
        begin_transition_id: "begin-rp2".into(),
        result_transition_id: "result-rp2".into(),
    };
    let DispatchEffect::Applied { permit, .. } = app
        .authorize_dispatch(prepared.handle, "dispatch-rp2", command)
        .unwrap()
    else {
        panic!("fresh dispatch expected")
    };
    let entry = app.consume_offline_initial(permit).unwrap();
    assert_eq!(entry.binding().campaign_root_id(), app.root_id());
    assert_eq!(entry.binding().session_id(), "session-rp2");
    assert_eq!(entry.binding().attempt_id(), identity.attempt_id);
    assert_eq!(entry.binding().acceptance_digest(), pin.digest());
    drop(entry);
    drop(app);
    let mut reopened = Campaign::open(config()).unwrap();
    reopened
        .install_offline_initial_authority(OfflineAcceptancePin::test_owned(&accepted_raw).unwrap())
        .unwrap();
    let state = reopened.read(&identity).unwrap().unwrap();
    assert_eq!(state.obligations[0].status, "executing");
    assert!(!matches!(
        reopened
            .prepare_initial(
                &identity,
                &state.revision,
                obligation,
                "prepare-after-restart"
            )
            .unwrap(),
        Preparation::Applied(_)
    ));
}
