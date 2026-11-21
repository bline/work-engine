//! The current binaries cannot reinterpret archival records or select offline cases.

use serde_json::Value;
use std::process::Command;
use work_engine_review_hosted::{messages_artifacts, revision::Profile, sha256_hex};

const R2_INPUTS: &[u8] = include_bytes!(
    "../../../../records/org-hosted-review-20261117/e2e/org-step3-live-1-inputs/job-inputs.json"
);
const R6_LIVE: &[u8] = include_bytes!(
    "../../../../records/org-hosted-review-20261117/e2e/org-step3-live-1.envelope-r6.json"
);
const R6_OFFLINE: &[u8] = include_bytes!(
    "../../../../records/org-hosted-review-20261117/e2e/org-step3-offline-1.envelope-r6.json"
);

#[test]
fn writer_refuses_both_archival_profiles_and_does_not_select_offline_cases() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("out");
    let invoke = |profile: &str| {
        Command::new(env!("CARGO_BIN_EXE_review-job-inputs"))
            .args([
                "--provider",
                profile,
                "--repo",
                "not-opened",
                "--commit",
                "unused",
                "--tree",
                "unused",
                "--patch-identity",
                "unused",
                "--job-id",
                "o7-refusal-test",
                "--max-requests",
                "8",
                "--max-reads",
                "16",
                "--out",
            ])
            .arg(&out)
            .output()
            .unwrap()
    };
    for profile in Profile::ALL {
        let refused = invoke(profile.provider());
        assert_eq!(refused.status.code(), Some(1));
        let detail = String::from_utf8(refused.stderr).unwrap();
        assert!(detail.contains(profile.revision().id), "{detail}");
        assert!(
            detail.contains("historical-only; use the exact archived"),
            "{detail}"
        );
        assert!(refused.stdout.is_empty());
    }
    for id in [
        "hosted-offline-exact-v1-responses",
        "hosted-offline-exact-v1-messages",
        messages_artifacts::REVISION,
    ] {
        let refused = invoke(id);
        assert_eq!(refused.status.code(), Some(1));
        assert!(
            String::from_utf8(refused.stderr)
                .unwrap()
                .contains("--provider must be one of")
        );
    }
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn checker_refuses_the_archival_record_before_loading_run_or_writing_an_envelope() {
    let dir = tempfile::tempdir().unwrap();
    let record = dir.path().join("record.json");
    let out = dir.path().join("out.json");
    std::fs::write(&record, R2_INPUTS).unwrap();
    let invoke = || {
        Command::new(env!("CARGO_BIN_EXE_review-result-check"))
            .arg("--inputs")
            .arg(&record)
            .args([
                "--run-inputs",
                "not-opened",
                "--job",
                "not-opened",
                "--state-dir",
                "not-opened",
                "--out",
            ])
            .arg(&out)
            .output()
            .unwrap()
    };
    let assert_archival = |id: &str| {
        let refused = invoke();
        assert_eq!(refused.status.code(), Some(2));
        assert!(refused.stdout.is_empty());
        let detail = String::from_utf8(refused.stderr).unwrap();
        assert!(detail.contains(id), "{detail}");
        assert!(
            detail.contains("historical-only; use the exact archived"),
            "{detail}"
        );
        assert!(!out.exists());
    };
    assert_archival("hosted-route-qualification-r2");
    // A historical R1 header is also refused; there is no metadata-based replay path.
    let mut value: Value = serde_json::from_slice(R2_INPUTS).unwrap();
    value["profile"] = "openai-codex".into();
    value["revision"] = "hosted-route-qualification-r1".into();
    std::fs::write(&record, serde_json::to_vec(&value).unwrap()).unwrap();
    assert_archival("hosted-route-qualification-r1");
    // A caller cannot select a test-only world by replacing revision/schema/standing.
    value["schema"] = "work-engine.review-hosted-offline-exact-inputs.v1".into();
    value["standing"] = "synthetic_inputs_only_not_an_admission".into();
    for revision in [
        "hosted-offline-exact-v1-responses",
        messages_artifacts::REVISION,
    ] {
        value["revision"] = revision.into();
        std::fs::write(&record, serde_json::to_vec(&value).unwrap()).unwrap();
        let refused = invoke();
        assert_eq!(refused.status.code(), Some(2));
        assert!(
            String::from_utf8(refused.stderr)
                .unwrap()
                .contains("no current hosted revision is selectable")
        );
        assert!(!out.exists());
    }
    // It also cannot overwrite an existing output by trying a historical recheck.
    std::fs::write(&out, b"untouched").unwrap();
    std::fs::write(&record, R2_INPUTS).unwrap();
    assert_eq!(invoke().status.code(), Some(2));
    assert_eq!(std::fs::read(&out).unwrap(), b"untouched");
}

#[test]
fn archive_keeps_both_r6_outcomes_and_standing() {
    for (bytes, hash, outcome) in [
        (
            R6_LIVE,
            "a422e68659a296f248eb26b9e9419c668fdba08533f265a1134d275bca6be55c",
            "checks_passed",
        ),
        (
            R6_OFFLINE,
            "e8ef199f1b4dc4355bd1a53a7efbb15b7fdf7db4d1bbad105145859a3833a831",
            "checks_failed",
        ),
    ] {
        assert_eq!(sha256_hex(bytes), hash);
        let envelope: Value = serde_json::from_slice(bytes).unwrap();
        assert_eq!(envelope["outcome"], outcome);
        assert_eq!(envelope["standing"], "owner_pending");
        assert_eq!(envelope["org_admission"], "none_requested");
        assert_eq!(envelope["consumable"], false);
        let checks = envelope["checks"].as_array().unwrap();
        let check = |id: &str| {
            checks.iter().find(|row| row["id"] == id).unwrap()["outcome"]
                .as_str()
                .unwrap()
        };
        if outcome == "checks_failed" {
            assert_eq!(check("strict_json"), "failed");
            assert_eq!(check("result_contract"), "not_run");
            assert_eq!(check("subject_equal"), "not_run");
        } else {
            assert_eq!(check("subject_equal"), "passed");
        }
    }
}
