use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use slice_campaign::codec::historical_digest;
use slice_campaign::contract::episode_digest;
use slice_campaign::{AcceptedBoundary, Baseline, CampaignIdentity, Effect, TrustedConfig};

// Each integration test compiles this shared module independently; admission
// uses the other helpers but not the history workload fixture below.
#[allow(dead_code)]
pub(crate) enum CorpusRoot {
    Random(tempfile::TempDir),
    Fixed(PathBuf),
}

impl CorpusRoot {
    #[allow(dead_code)]
    pub(crate) fn path(&self) -> &Path {
        match self {
            Self::Random(temp) => temp.path(),
            Self::Fixed(path) => path,
        }
    }
}

impl Drop for CorpusRoot {
    fn drop(&mut self) {
        if let Self::Fixed(path) = self {
            fs::remove_dir_all(path).unwrap();
        }
    }
}

/// Build a controlled SC0 campaign through its public API for history workloads.
/// The caller owns the returned temporary root for the whole measurement.
#[allow(dead_code)]
pub(crate) fn selected_campaign(
    specialist_count: usize,
    all_selected: bool,
) -> (
    CorpusRoot,
    slice_campaign::Campaign,
    CampaignIdentity,
    slice_campaign::Snapshot,
    Box<dyn Fn() -> TrustedConfig>,
) {
    use slice_campaign::{AdmitRequest, AdvancePhase, Campaign, Consequence};
    let (temp, repo, workspace, identity, baseline, boundary) =
        if let Ok(fixed) = std::env::var("P1_CORPUS_ROOT") {
            let path = PathBuf::from(fixed);
            assert!(path.is_absolute());
            fs::create_dir(&path).unwrap();
            let (repo, workspace, identity, baseline, boundary) = setup_paths(&path);
            (
                CorpusRoot::Fixed(path),
                repo,
                workspace,
                identity,
                baseline,
                boundary,
            )
        } else {
            let (temp, repo, workspace, identity, baseline, boundary) = setup();
            (
                CorpusRoot::Random(temp),
                repo,
                workspace,
                identity,
                baseline,
                boundary,
            )
        };
    let initial_configuration = config(
        temp.path().join("campaign"),
        repo.clone(),
        workspace.clone(),
        identity.clone(),
        baseline.clone(),
        boundary.clone(),
    );
    let reopen_root = temp.path().join("campaign");
    let reopen_repo = repo.clone();
    let reopen_workspace = workspace.clone();
    let reopen_identity = identity.clone();
    let reopen_baseline = baseline.clone();
    let reopen_boundary = boundary.clone();
    let reopen = Box::new(move || {
        config(
            reopen_root.clone(),
            reopen_repo.clone(),
            reopen_workspace.clone(),
            reopen_identity.clone(),
            reopen_baseline.clone(),
            reopen_boundary.clone(),
        )
    });
    let mut app = Campaign::initialize(initial_configuration).unwrap();
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
    if specialist_count == 0 {
        return (temp, app, identity, ready, reopen);
    }
    let candidate = ready.candidate.as_ref().unwrap();
    let subject = json!({"commit":candidate.commit,"tree":candidate.tree,
        "patchIdentity":candidate.patch_identity});
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
    (temp, app, identity, selected, reopen)
}

pub(crate) fn fixture(hash: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/vectors")
            .join(format!("{hash}.json")),
    )
    .unwrap()
}
pub(crate) fn setup() -> (
    tempfile::TempDir,
    PathBuf,
    PathBuf,
    CampaignIdentity,
    Baseline,
    AcceptedBoundary,
) {
    let temp = tempfile::tempdir().unwrap();
    let (repo, workspace, identity, baseline, boundary) = setup_paths(temp.path());
    (temp, repo, workspace, identity, baseline, boundary)
}

fn setup_paths(
    root: &Path,
) -> (
    PathBuf,
    PathBuf,
    CampaignIdentity,
    Baseline,
    AcceptedBoundary,
) {
    let repo = root.join("repo");
    let workspace = root.join("workspace");
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
    (repo, workspace, identity, baseline, boundary)
}
pub(crate) fn config(
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
pub(crate) fn anchored_artifacts(repo: &Path) -> (Vec<u8>, Vec<u8>) {
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
pub(crate) fn claim(
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
pub(crate) fn applied<T>(effect: Effect<T>) -> T {
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
