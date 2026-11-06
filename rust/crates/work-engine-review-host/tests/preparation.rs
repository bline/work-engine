use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

fn refuse(input: &Path, output: &Path) -> String {
    let result = Command::new(env!("CARGO_BIN_EXE_work-engine-review-host"))
        .args(["prepare", "--input"])
        .arg(input)
        .arg("--out")
        .arg(output)
        .output()
        .unwrap();
    assert!(!result.status.success(), "unexpected preparation success");
    assert!(!output.exists(), "refusal created proposal effects");
    String::from_utf8_lossy(&result.stderr).into_owned()
}

#[test]
fn preparation_refuses_malformed_oversized_relative_and_symlink_inputs() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("input.json");
    let output = temp.path().join("proposal");
    fs::write(&input, b"{").unwrap();
    refuse(&input, &output);
    fs::write(&input, vec![b' '; 1024 * 1024 + 1]).unwrap();
    assert!(refuse(&input, &output).contains("exceeds1MiB"));
    refuse(Path::new("relative.json"), &output);
    let alias = temp.path().join("alias.json");
    std::os::unix::fs::symlink(&input, &alias).unwrap();
    assert!(refuse(&alias, &output).contains("canonical"));
}

/// Explicit offline qualification input, never installed or admitted. The
/// ignored test must be selected with a genuine private proposal input path.
#[test]
#[ignore = "requires PREPARATION_TEST_INPUT exact genuine proposal data"]
fn preparation_refuses_authority_and_standing_substitutions_before_effects() {
    let selected = std::env::var("PREPARATION_TEST_INPUT").expect("explicit input required");
    let original: Value = serde_json::from_slice(&fs::read(selected).unwrap()).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let replacement = temp.path().join("replacement");
    fs::write(&replacement, b"#!/bin/sh\nexit 77\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&replacement, fs::Permissions::from_mode(0o755)).unwrap();
    let replacement_pin = json!({"path":replacement,"sha256":claim_evidence::sha256_bytes(&fs::read(&replacement).unwrap()),"mode":0o755,"ownerUid":rustix::process::geteuid().as_raw()});
    for case in [
        "standing",
        "fixture",
        "unknown-authority",
        "missing-source",
        "changed-source",
        "missing-episode",
        "repository",
        "unknown-launcher",
        "unknown-bwrap",
        "unknown-cbm",
        "unknown-client",
    ] {
        let mut input = original.clone();
        match case {
            "standing" => input["standing"] = json!("approved"),
            "fixture" => input["startup"]["profile"] = json!("fixture"),
            "unknown-authority" => input["approved"] = json!(true),
            "missing-source" => {
                input["startup"]["base"]["claims"]["source"]["path"] =
                    json!(temp.path().join("missing"))
            }
            "changed-source" => {
                input["startup"]["base"]["claims"]["source"]["sha256"] = json!("0".repeat(64))
            }
            "missing-episode" => {
                input["startup"]["base"]["episodeAuthority"]["path"] =
                    json!(temp.path().join("missing"))
            }
            "unknown-launcher" => {
                input["startup"]["qualified"]["cbmLauncher"] = replacement_pin.clone()
            }
            "unknown-bwrap" => input["startup"]["qualified"]["bwrap"] = replacement_pin.clone(),
            "unknown-cbm" => input["startup"]["qualified"]["cbm"] = replacement_pin.clone(),
            "unknown-client" => {
                input["startup"]["qualified"]["cbmClient"] = replacement_pin.clone()
            }
            "repository" => input["policy"]["repository"] = json!(temp.path()),
            _ => unreachable!(),
        }
        let path = temp.path().join(format!("{case}.json"));
        fs::write(&path, serde_json::to_vec(&input).unwrap()).unwrap();
        let message = refuse(&path, &temp.path().join(format!("out-{case}")));
        if case.starts_with("unknown-") && case != "unknown-authority" {
            assert!(message.contains("independently reviewed"), "{message}");
        }
    }
}

#[test]
#[ignore = "requires PREPARATION_TEST_POLICY exact uninstalled output"]
fn emitted_policy_passes_existing_loader_and_refuses_historical_template() {
    use slice_campaign::job_admission::{JobReviewPolicy, JobReviewPolicyDocument};
    use std::os::unix::fs::PermissionsExt;
    let path = std::env::var("PREPARATION_TEST_POLICY").expect("explicit policy required");
    let document: JobReviewPolicyDocument =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    document.validate_proposal().unwrap();
    JobReviewPolicy::open_fixture(Path::new(&path)).unwrap();
    let mut changed = serde_json::to_value(&document).unwrap();
    changed["executionTemplate"]["identity"]["attemptId"] = json!("different-attempt");
    let temp = tempfile::tempdir().unwrap();
    let wrong = temp.path().join("wrong.json");
    fs::write(&wrong, serde_json::to_vec(&changed).unwrap()).unwrap();
    fs::set_permissions(&wrong, fs::Permissions::from_mode(0o400)).unwrap();
    assert!(JobReviewPolicy::open_fixture(&wrong).is_err());
}
