#![cfg(feature = "test-support")]
#[path = "../../slice-campaign/tests/support/mod.rs"]
mod support;

use serde_json::json;
use slice_campaign::{
    AcceptedExecutionSelection, AdmitRequest, AdvancePhase, Campaign, Consequence, DispatchCommand,
    DispatchEffect, Preparation, TrustedConfig,
};
use std::os::unix::fs::MetadataExt;

#[tokio::test]
async fn fixture_job_admission_reopens_and_feeds_exact_consumer_without_launch() {
    run("valid").await;
}
#[tokio::test]
async fn revoked_policy_is_refused_by_consumer_after_dispatch() {
    run("revoked").await;
}
#[tokio::test]
async fn fixture_authority_cannot_execute_qualified_provider() {
    run("launch-refused").await;
}
#[tokio::test]
async fn changed_consumer_session_is_refused() {
    run("session-mismatch").await;
}
#[tokio::test]
async fn forged_private_admission_is_refused_by_consumer() {
    run("forged").await;
}
#[tokio::test]
async fn replaced_policy_is_refused_by_consumer() {
    run("policy-replaced").await;
}
#[tokio::test]
async fn caller_cannot_weaken_policy_claim_contracts() {
    run("weaker-selection").await;
}
#[tokio::test]
async fn receipt_artifact_mismatch_refuses_campaign_installation() {
    run("receipt-mismatch").await;
}
#[tokio::test]
async fn physical_profile_artifact_mismatch_refuses_campaign_installation() {
    run("profile-artifact-mismatch").await;
}
#[tokio::test]
async fn matching_writer_outside_admitted_evidence_root_is_refused() {
    run("wrong-evidence-root").await;
}
async fn run(mode: &str) {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let (bare_repo, _initial_workspace, identity, baseline, boundary) =
        support::setup_paths(temp.path());
    let repo = temp.path().join("candidate");
    assert!(
        std::process::Command::new("git")
            .args(["clone", "--quiet", "--no-checkout"])
            .arg(&bare_repo)
            .arg(&repo)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        std::process::Command::new("git")
            .arg("-C")
            .arg(&repo)
            .arg("fetch")
            .arg("--quiet")
            .arg(&bare_repo)
            .arg("refs/work-engine/*:refs/work-engine/*")
            .status()
            .unwrap()
            .success()
    );
    assert!(
        std::process::Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args([
                "checkout",
                "--quiet",
                "--detach",
                "49624730af1663f50a3ff85abfdd1b90647d0cdb"
            ])
            .status()
            .unwrap()
            .success()
    );
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
    let mut selection = json!({"schemaVersion":2,"owner":"slice-supervisor","selectionId":"selection-rp2",
        "subject":subject,"specialists":[{"obligationId":obligation,"skill":"implementation-review",
        "selection":"selected","requiredClaims":[builder,campaign_claim]}]});
    let selected =
        slice_campaign::contract::validate_selection(&selection, &identity, &checked.reference)
            .unwrap();
    use review_execution_evidence::claude::{
        CredentialSource, PinnedProgram, QualifiedScope, QualifiedSelection, QualifiedWriter,
        TrustedClaudeProfile, compose_applied_initial_prompt,
    };
    use review_execution_evidence::{ExecutionBinding, ExecutionProvenance};
    use slice_campaign::job_admission::{
        JobAdmissionEffect, JobReviewPolicy, JobReviewPolicyDocument, JobReviewRequest,
    };
    use std::{fs, time::Duration};
    let program_path = temp.path().join("fixture-program-never-launched");
    fs::copy(std::env::current_exe().unwrap(), &program_path).unwrap();
    fs::set_permissions(&program_path, fs::Permissions::from_mode(0o700)).unwrap();
    let program =
        PinnedProgram::load(&program_path, &sha(&fs::read(&program_path).unwrap())).unwrap();
    let peer_profile = TrustedClaudeProfile::load_offline_guarded_peer(
        program.clone(),
        program,
        "fixture-model",
        "high",
        vec!["fixture-model".into()],
        b"{\"fixture\":true}",
    )
    .unwrap();
    let credential_path = temp.path().join("empty-fixture-credential");
    fs::write(&credential_path, b"{}").unwrap();
    fs::set_permissions(&credential_path, fs::Permissions::from_mode(0o600)).unwrap();
    // Metadata only; no credential bytes or tokens are read.
    let credential = CredentialSource::new(&credential_path).unwrap();
    let template = b"fixture job review template";
    let accepted = json!({
        "schemaVersion":1,"decision":"accepted","reference":"offline-rp2-test","revision":"revision-1",
        "identity":identity,"obligationId":obligation,"candidateTree":checked.reference.tree,
        "builderClaimId":selection["specialists"][0]["requiredClaims"][0]["claimId"],
        "campaignClaimId":selection["specialists"][0]["requiredClaims"][1]["claimId"],
        "selectionDigest":selected.campaign_digest,"physicalProfileDigest":checked.profile_digest,
        "claudeProfileDigest":peer_profile.profile_digest(),"programSha256":peer_profile.program().sha256(),
        "guardSha256":peer_profile.guard().unwrap().sha256(),"cbmStoreSha256":"d".repeat(64),
        "catalogSha256":peer_profile.catalog_sha256(),"requestedModel":peer_profile.requested_model(),
        "credentialSourceMetadataSha256":credential.metadata_digest(),
        "resourcePolicy":"strict_aggregate_session_64mib_v1",
        "sessionId":"session-rp2","attemptId":identity.attempt_id,
        "acceptedBoundarySha256":boundary.sha256,
        "promptTemplateSha256":sha(template),"attemptHorizonMillis":60_000,
    });
    let policy_path = temp.path().join("fixture-policy.json");
    let policy_doc = JobReviewPolicyDocument {
        schema_version: 1,
        policy_id: "fixture-job-policy".into(),
        revision: "v1".into(),
        active: true,
        expires_at_millis: u64::MAX,
        admitted_uid: std::fs::metadata(temp.path()).unwrap().uid(),
        repository: repo.clone(),
        jobs_root: temp.path().to_path_buf(),
        startup_authority: slice_campaign::job_admission::JobStartupAuthority {
            claims_source_sha256: "b".repeat(64),
            claims_configuration_sha256: "c".repeat(64),
            completion_consumer: "fixture-completion".into(),
        },
        execution_template: accepted.clone(),
        review_selection_template: selection.clone(),
    };
    fs::write(&policy_path, serde_json::to_vec(&policy_doc).unwrap()).unwrap();
    fs::set_permissions(&policy_path, fs::Permissions::from_mode(0o600)).unwrap();
    let policy = JobReviewPolicy::open_fixture(&policy_path).unwrap();
    let mut job = JobReviewRequest {
        identity: identity.clone(),
        candidate: checked.reference.clone(),
        physical_profile_digest: checked.profile_digest.clone(),
        cbm_store_sha256: "d".repeat(64),
        session_id: "session-rp2".into(),
    };
    if mode == "receipt-mismatch" {
        job.candidate.receipt_sha256 = "1".repeat(64);
    }
    if mode == "profile-artifact-mismatch" {
        job.candidate.physical_profile_sha256 = "2".repeat(64);
    }
    let job_dir = temp.path().join(job.directory_name().unwrap());
    fs::create_dir(&job_dir).unwrap();
    fs::set_permissions(&job_dir, fs::Permissions::from_mode(0o700)).unwrap();
    let workspace = job_dir.join("workspace");
    fs::create_dir(&workspace).unwrap();
    let JobAdmissionEffect::Committed(pin) = policy.admit(&job).unwrap() else {
        panic!("fresh admission expected")
    };
    assert!(matches!(
        policy.admit(&job).unwrap(),
        JobAdmissionEffect::Replayed(_)
    ));
    assert_eq!(pin.digest(), policy.reopen(&job).unwrap().digest());
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
    let config = || {
        TrustedConfig::qualified_initial_v4(
            job_dir.join("campaign"),
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
    if mode == "weaker-selection" {
        for claim in selection["specialists"][0]["requiredClaims"]
            .as_array_mut()
            .unwrap()
        {
            claim["profile"]["integrityRequired"] = json!(false);
            let mut revision = claim.clone();
            revision.as_object_mut().unwrap().remove("revision");
            let parsed = claim_evidence::codec::parse_json(&revision.to_string()).unwrap();
            claim["revision"] = json!(format!(
                "production-path-claim-revision-v1@{}",
                claim_evidence::codec::digest(&parsed).unwrap()
            ));
        }
    }
    let selected_state = support::applied(
        app.bind_selection(&identity, &ready.revision, "selection-rp2", selection)
            .unwrap(),
    );
    if mode == "weaker-selection" {
        assert!(
            app.install_qualified_initial_authority(policy.reopen(&job).unwrap())
                .is_err()
        );
        return;
    }
    if mode == "receipt-mismatch" || mode == "profile-artifact-mismatch" {
        let error = app
            .install_qualified_initial_authority(policy.reopen(&job).unwrap())
            .unwrap_err();
        assert!(
            error.to_string().contains("job candidate artifacts differ"),
            "{error}"
        );
        return;
    }
    app.install_qualified_initial_authority(policy.reopen(&job).unwrap())
        .unwrap();
    assert!(
        app.install_qualified_initial_authority(policy.reopen(&job).unwrap())
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
    let prepared_request = prepared.request.clone();
    let episode = command.episode_id.clone();
    let DispatchEffect::Applied { permit, receipt } = app
        .authorize_dispatch(prepared.handle, "dispatch-rp2", command)
        .unwrap()
    else {
        panic!("fresh dispatch expected")
    };
    let entry = app.consume_qualified_initial(permit).unwrap();
    assert_eq!(entry.binding().campaign_root_id(), app.root_id());
    assert_eq!(entry.binding().session_id(), "session-rp2");
    assert_eq!(entry.binding().attempt_id(), identity.attempt_id);
    assert_eq!(entry.binding().acceptance_digest(), pin.digest());
    assert!(entry.is_fixture());
    let prompt = compose_applied_initial_prompt(template, &prepared_request).unwrap();
    let scope = QualifiedScope::bind_offline_peer(
        peer_profile.clone(),
        QualifiedSelection {
            root_id: "fixture-job-evidence".into(),
            provenance: ExecutionProvenance {
                binding: ExecutionBinding {
                    campaign_root_id: app.root_id().into(),
                    obligation_id: obligation.into(),
                    candidate_digest: prepared_request.candidate.receipt_sha256.clone(),
                    selection_digest: prepared_request.selection.campaign_digest.clone(),
                    profile_digest: prepared_request.profile_digest.clone(),
                    execution_profile_digest: Some(peer_profile.profile_digest().into()),
                    prepared_request_sha256: prepared_request.request_digest.clone(),
                    review_episode_id: episode,
                    attempt_id: identity.attempt_id.clone(),
                },
                dispatch_operation_id: "dispatch-rp2".into(),
                dispatch_revision: receipt.result_revision.as_str().into(),
                execution_session_id: Some(
                    if mode == "session-mismatch" {
                        "changed-session"
                    } else {
                        "session-rp2"
                    }
                    .into(),
                ),
            },
            selection_id: prepared_request.selection.selection_id.clone(),
            candidate_commit: checked.reference.commit.clone(),
            candidate_tree: checked.reference.tree.clone(),
            candidate_patch_identity: checked.reference.patch_identity.clone(),
            candidate_path: repo.clone(),
            candidate_tree_sha256: tree_sha(&repo),
            cbm_project: "fixture-index".into(),
            cbm_store_sha256: "d".repeat(64),
            cbm_generation: "fixture-generation".into(),
            cbm_coverage_sha256: "e".repeat(64),
            prompt,
            subject: claim_evidence::codec::parse_json(&subject.to_string()).unwrap(),
            credential: Some(credential),
            offline_witness: None,
            horizon: Duration::from_secs(60),
        },
    )
    .unwrap()
    .with_applied_physical_profile(&checked.profile_digest)
    .unwrap()
    .with_applied_request(template, prepared_request)
    .unwrap();
    let evidence_root = if mode == "wrong-evidence-root" {
        temp.path().join("fixture-evidence")
    } else {
        job_dir.join("evidence")
    };
    let mut writer = QualifiedWriter::create(&evidence_root, scope).unwrap();
    if mode == "wrong-evidence-root" {
        let error = writer.check_fixture_job_entry(entry).unwrap_err();
        assert!(
            error.to_string().contains("job evidence root differs"),
            "{error}"
        );
        assert!(!evidence_root.join("session-live").exists());
        return;
    }
    if mode == "forged" {
        let path = job_dir.join("review-admission.json");
        let mut forged: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        forged["acceptance"]["requestedModel"] = json!("caller-forged-model");
        fs::write(&path, serde_json::to_vec(&forged).unwrap()).unwrap();
        assert!(writer.check_fixture_job_entry(entry).is_err());
        assert!(!evidence_root.join("session-live").exists());
        return;
    }
    if mode == "policy-replaced" {
        let mut replaced = policy_doc.clone();
        replaced.revision = "replacement-v2".into();
        fs::write(&policy_path, serde_json::to_vec(&replaced).unwrap()).unwrap();
        assert!(writer.check_fixture_job_entry(entry).is_err());
        assert!(!evidence_root.join("session-live").exists());
        return;
    }
    if mode == "revoked" {
        let mut revoked = policy_doc.clone();
        revoked.active = false;
        fs::write(&policy_path, serde_json::to_vec(&revoked).unwrap()).unwrap();
        assert!(writer.check_fixture_job_entry(entry).is_err());
        assert!(!evidence_root.join("session-live").exists());
        return;
    }
    if mode == "launch-refused" {
        assert!(writer.execute_applied_initial(entry).await.is_err());
        assert!(!evidence_root.join("session-live").exists());
        return;
    }
    if mode == "session-mismatch" {
        assert!(writer.check_fixture_job_entry(entry).is_err());
        assert!(!evidence_root.join("session-live").exists());
        return;
    }
    writer.check_fixture_job_entry(entry).unwrap();
    // The consumer checked the real exact binding; no execute method or process
    // was invoked, and no result/evidence was manufactured.
    assert!(!evidence_root.join("session-live").exists());
    drop(app);
    let mut reopened = Campaign::open(config()).unwrap();
    reopened
        .install_qualified_initial_authority(policy.reopen(&job).unwrap())
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

fn sha(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
fn tree_sha(root: &std::path::Path) -> String {
    use sha2::{Digest, Sha256};
    let mut paths = vec![];
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for item in std::fs::read_dir(&dir).unwrap() {
            let item = item.unwrap();
            if dir == root && item.file_name() == ".git" {
                continue;
            }
            let p = item.path();
            if p.is_dir() {
                stack.push(p)
            } else {
                paths.push(p)
            }
        }
    }
    paths.sort();
    let mut h = Sha256::new();
    for p in paths {
        let n = p.strip_prefix(root).unwrap().to_str().unwrap();
        let b = std::fs::read(&p).unwrap();
        h.update((n.len() as u64).to_be_bytes());
        h.update(n.as_bytes());
        h.update((b.len() as u64).to_be_bytes());
        h.update(b);
    }
    format!("{:x}", h.finalize())
}
