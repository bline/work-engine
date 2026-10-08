use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use slice_campaign::codec::{historical_digest, raw_sha256};
use slice_campaign::{Baseline, CampaignIdentity, verify_existing_subject};

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/vectors")
            .join(name),
    )
    .unwrap()
}
fn vector(hash: &str) -> Vec<u8> {
    fixture(&format!("{hash}.json"))
}
fn setup() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let repo = temp.path().join("repo");
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
    (temp, repo)
}
fn trusted_gate() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    (
        vector("f52792e22d2a1b73e007bc7078607d6657989248a35de1371a1180d23101d206"),
        vector("d155cc2ac1cf1ab26b0e186d6a6ac86f547b1a0b9c3ce74f0ba69eecd37a2f34"),
        vector("c927b51b887e37f6fdc02e1fabfadddd211d0ac30712cbc6c5028993ad3889c2"),
    )
}
fn reanchor(repo: &Path, candidate: &mut Value, profile: &mut Value) {
    candidate["repository"] = json!(repo.to_str().unwrap());
    profile["subject"]["repository"] = json!(repo.to_str().unwrap());
    profile["subject_digest"] = json!(historical_digest(&profile["subject"]).unwrap());
    profile["provenance"]["derivation_sources"]["checkpoint_subject"]["identity"]["subject_digest"] =
        profile["subject_digest"].clone();
    let mut unsigned = profile.clone();
    unsigned.as_object_mut().unwrap().remove("profile_digest");
    profile["profile_digest"] = json!(historical_digest(&unsigned).unwrap());
}
fn identity() -> CampaignIdentity {
    CampaignIdentity {
        run_id: "sc0-oracle-c2".into(),
        slice_number: 1,
        attempt_id: "sc0-attempt".into(),
        plan_version: "sc0-controlled-v1".into(),
    }
}
fn baseline() -> Baseline {
    Baseline {
        accepted_commit: "a8e074d765d94a7e816b7ab49db428fd4a760a9f".into(),
        accepted_tree: "17868afbf46ea53715981fce83171001c632b957".into(),
        inter_slice_commit: "a8e074d765d94a7e816b7ab49db428fd4a760a9f".into(),
    }
}

#[test]
fn real_git_subject_and_frozen_gate_are_bound() {
    let (_temp, repo) = setup();
    let mut candidate: Value = serde_json::from_slice(&vector(
        "4a2c4f1872f6dfe8de275181f08b1e4d8c33a437c2519cd8b9ce8b3c9b56a5cd",
    ))
    .unwrap();
    let mut profile: Value = serde_json::from_slice(&vector(
        "5f78af676bf50a7a328b12e0ddcbf619ecfd78fd8e2f46f299330ff0d96095db",
    ))
    .unwrap();
    reanchor(&repo, &mut candidate, &mut profile);
    let (gate, manifest, receipt) = trusted_gate();
    let candidate_raw = serde_json::to_vec(&candidate).unwrap();
    let profile_raw = serde_json::to_vec(&profile).unwrap();
    let verified = verify_existing_subject(
        &repo,
        &identity(),
        &baseline(),
        &candidate_raw,
        &profile_raw,
        &gate,
        &manifest,
        &receipt,
    )
    .unwrap();
    assert_eq!(
        verified.reference.receipt_sha256,
        raw_sha256(&candidate_raw)
    );
    assert_eq!(
        verified.reference.tree,
        "882023b5f0d6b89425f151aab85e6534a53cf5bb"
    );
    let mut wrong = receipt.clone();
    wrong.push(b' ');
    assert!(
        verify_existing_subject(
            &repo,
            &identity(),
            &baseline(),
            &candidate_raw,
            &profile_raw,
            &gate,
            &manifest,
            &wrong
        )
        .is_err()
    );
    let mut wrong_candidate = candidate.clone();
    wrong_candidate["task_patch_digest"] = json!("0".repeat(64));
    assert!(
        verify_existing_subject(
            &repo,
            &identity(),
            &baseline(),
            &serde_json::to_vec(&wrong_candidate).unwrap(),
            &profile_raw,
            &gate,
            &manifest,
            &receipt
        )
        .is_err()
    );
}

#[test]
fn sc0_candidate_negative_vectors_reject() {
    let (_temp, repo) = setup();
    let manifest: Value = serde_json::from_slice(
        &fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/subject-handoff.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut profile: Value = serde_json::from_slice(&vector(
        "5f78af676bf50a7a328b12e0ddcbf619ecfd78fd8e2f46f299330ff0d96095db",
    ))
    .unwrap();
    let (gate, gate_manifest, gate_receipt) = trusted_gate();
    for case in manifest["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["kind"] == "candidate")
    {
        let file = case["artifact"]["file"].as_str().unwrap();
        let mut candidate: Value =
            serde_json::from_slice(&fixture(file.trim_start_matches("vectors/"))).unwrap();
        reanchor(&repo, &mut candidate, &mut profile);
        let actual = verify_existing_subject(
            &repo,
            &identity(),
            &baseline(),
            &serde_json::to_vec(&candidate).unwrap(),
            &serde_json::to_vec(&profile).unwrap(),
            &gate,
            &gate_manifest,
            &gate_receipt,
        )
        .is_ok();
        assert_eq!(
            actual,
            case["sc1_supported_profile_expected_accept"]
                .as_bool()
                .unwrap(),
            "{}",
            case["case"]
        );
    }
}

#[test]
fn redigested_profile_field_types_and_overflow_are_bounded() {
    let (_temp, repo) = setup();
    let candidate: Value = serde_json::from_slice(&vector(
        "4a2c4f1872f6dfe8de275181f08b1e4d8c33a437c2519cd8b9ce8b3c9b56a5cd",
    ))
    .unwrap();
    let profile: Value = serde_json::from_slice(&vector(
        "5f78af676bf50a7a328b12e0ddcbf619ecfd78fd8e2f46f299330ff0d96095db",
    ))
    .unwrap();
    let (gate, manifest, receipt) = trusted_gate();
    let verify = |mut candidate: Value, mut profile: Value| {
        reanchor(&repo, &mut candidate, &mut profile);
        verify_existing_subject(
            &repo,
            &identity(),
            &baseline(),
            &serde_json::to_vec(&candidate).unwrap(),
            &serde_json::to_vec(&profile).unwrap(),
            &gate,
            &manifest,
            &receipt,
        )
    };
    assert!(verify(candidate.clone(), profile.clone()).is_ok());
    let mut bad = profile.clone();
    bad["limitations"] = json!([17]);
    assert!(verify(candidate.clone(), bad).is_err());
    let mut bad = profile.clone();
    bad["subject"]["limitations"] = json!([17]);
    assert!(verify(candidate.clone(), bad).is_err());
    let mut bad_candidate = candidate.clone();
    let mut bad = profile.clone();
    bad_candidate["created_at"] = json!(17);
    bad["subject"]["evidence_cutoff"] = json!(17);
    assert!(verify(bad_candidate, bad).is_err());
    let mut large_binary = profile.clone();
    let files = large_binary["observations"]["files"]["value"]
        .as_array_mut()
        .unwrap();
    let mut changed = 0;
    for row in files {
        if row["binary"] == false && changed < 2 {
            row["additions"] = json!(u64::MAX);
            changed += 1;
        }
    }
    assert_eq!(changed, 2);
    assert!(verify(candidate.clone(), large_binary).is_ok());
    let mut text_only = profile;
    let files = text_only["observations"]["files"]["value"]
        .as_array_mut()
        .unwrap();
    for row in files.iter_mut() {
        row["binary"] = json!(false);
        row["additions"] = json!(u64::MAX);
        row["deletions"] = json!(0);
    }
    assert!(verify(candidate, text_only).is_err());
}
