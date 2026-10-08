use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use slice_campaign::codec::historical_digest;
use slice_campaign::contract::episode_digest;
use slice_campaign::{
    AcceptedBoundary, AdmitRequest, AdvancePhase, Baseline, Campaign, CampaignIdentity,
    Consequence, Effect, Preparation, Reconcile, TrustedConfig,
};

fn fixture(hash: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/vectors")
            .join(format!("{hash}.json")),
    )
    .unwrap()
}
fn setup() -> (
    tempfile::TempDir,
    PathBuf,
    PathBuf,
    CampaignIdentity,
    Baseline,
    AcceptedBoundary,
) {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
    let workspace = temp.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    let bundle=Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vectors/4a4b1ad328857904b2aa50ecc7a3206d3d0e5962f88b69f3d12b703c33b99d45.bundle");
    assert!(
        Command::new("/usr/bin/git")
            .args(["init", "--bare", "-q"])
            .arg(&repo)
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new("/usr/bin/git")
            .arg("-C")
            .arg(&repo)
            .args(["fetch", "-q"])
            .arg(bundle)
            .args(["refs/*:refs/*"])
            .status()
            .unwrap()
            .success()
    );
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
    (temp, repo, workspace, identity, baseline, boundary)
}
fn config(
    root: PathBuf,
    repo: PathBuf,
    workspace: PathBuf,
    identity: CampaignIdentity,
    baseline: Baseline,
    boundary: AcceptedBoundary,
) -> TrustedConfig {
    TrustedConfig::controlled_sc0(
        root,
        repo,
        workspace,
        identity,
        boundary,
        baseline,
        "hp3-host-writer".into(),
        fixture("f52792e22d2a1b73e007bc7078607d6657989248a35de1371a1180d23101d206"),
        fixture("d155cc2ac1cf1ab26b0e186d6a6ac86f547b1a0b9c3ce74f0ba69eecd37a2f34"),
        fixture("c927b51b887e37f6fdc02e1fabfadddd211d0ac30712cbc6c5028993ad3889c2"),
    )
    .unwrap()
}
fn anchored_artifacts(repo: &Path) -> (Vec<u8>, Vec<u8>) {
    let mut candidate: Value = serde_json::from_slice(&fixture(
        "4a2c4f1872f6dfe8de275181f08b1e4d8c33a437c2519cd8b9ce8b3c9b56a5cd",
    ))
    .unwrap();
    let mut profile: Value = serde_json::from_slice(&fixture(
        "5f78af676bf50a7a328b12e0ddcbf619ecfd78fd8e2f46f299330ff0d96095db",
    ))
    .unwrap();
    candidate["repository"] = json!(repo.to_str().unwrap());
    profile["subject"]["repository"] = json!(repo.to_str().unwrap());
    profile["subject_digest"] = json!(historical_digest(&profile["subject"]).unwrap());
    profile["provenance"]["derivation_sources"]["checkpoint_subject"]["identity"]["subject_digest"] =
        profile["subject_digest"].clone();
    let mut unsigned = profile.clone();
    unsigned.as_object_mut().unwrap().remove("profile_digest");
    profile["profile_digest"] = json!(historical_digest(&unsigned).unwrap());
    (
        serde_json::to_vec(&candidate).unwrap(),
        serde_json::to_vec(&profile).unwrap(),
    )
}
fn claim_digest(value: &Value) -> String {
    let encoded = serde_json::to_string(value).unwrap();
    let value = claim_evidence::codec::parse_json(&encoded).unwrap();
    claim_evidence::codec::digest(&value).unwrap()
}
fn claim(
    identity: &CampaignIdentity,
    obligation: &str,
    candidate: &Value,
    boundary: &str,
) -> Value {
    let consumer = if boundary == "builder_projection" {
        format!("slice-builder:{}", identity.key())
    } else {
        format!("slice-campaign:{}", identity.key())
    };
    let episode_id =
        episode_digest(&json!({"identity":identity,"obligationId":obligation})).unwrap();
    let mut claim = json!({"schemaVersion":1,"claimId":"","revision":"","proposition":"independent initial review","subject":{"candidate":candidate,"reviewEpisodeId":&episode_id[..32]},"coveredState":"initial review","consumptionBoundary":boundary,"consumer":consumer,"acceptance":{"owner":"operator","source":"accepted plan","unestablishedRoute":"re-review"},"profile":{"id":"production-path-v1","revision":"production-path-profile-v1","allowedMechanisms":["direct"],"admissibleObservers":["host"],"integrityRequired":true,"requiredRealization":"claude-sonnet-5","requiredCapabilities":[],"continuity":"fresh_initial"}});
    let id_body = json!({"proposition":claim["proposition"],"subject":claim["subject"],"coveredState":claim["coveredState"],"consumptionBoundary":claim["consumptionBoundary"],"consumer":claim["consumer"]});
    claim["claimId"] = json!(format!(
        "production-path-claim-v1@{}",
        claim_digest(&id_body)
    ));
    let mut revision_body = claim.clone();
    revision_body.as_object_mut().unwrap().remove("revision");
    claim["revision"] = json!(format!(
        "production-path-claim-revision-v1@{}",
        claim_digest(&revision_body)
    ));
    claim
}
fn applied<T>(effect: Effect<T>) -> T {
    match effect {
        Effect::Applied { value, .. } => value,
        other => panic!(
            "expected applied, got {}",
            match other {
                Effect::Replayed { .. } => "replay",
                Effect::NoEffect(_) => "no effect",
                Effect::OutcomeUnknown(_) => "unknown",
                _ => unreachable!(),
            }
        ),
    }
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
    let reopened = Campaign::open(config(
        root,
        repo,
        workspace,
        identity.clone(),
        baseline,
        boundary,
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
            serde_json::from_slice(&fs::read(root.join(".campaign-native-initial-v1")).unwrap())
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
