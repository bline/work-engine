#![cfg(unix)]
use std::process::Command;
#[cfg(all(feature = "test-support", feature = "test-peer"))]
mod support;

#[test]
fn completion_startup_cannot_override_installed_authority() {
    let host = env!("CARGO_BIN_EXE_work-engine-review-host");
    for operation in ["completion-bootstrap", "completion-serve"] {
        let output = Command::new(host)
            .args([
                operation,
                "--startup",
                "/missing",
                "--completion-selection",
                "/caller/authority.json",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("unsupported host startup option")
        );
    }
}

#[cfg(all(feature = "test-support", feature = "test-peer"))]
mod cold_fixture {
    use super::*;

    #[test]
    fn cold_completion_reopens_exact_owners_without_launch_or_result_reissue() {
        let fixture = support::Fixture::new(0, "result");
        let initialized = fixture.initialize();
        let revision = initialized["campaign"]["revision"].as_str().unwrap();
        let mut review = fixture.start(&[]);
        review.ready();
        review.domain("execute_initial", revision);
        let result = review.receive();
        assert_eq!(
            result["result"]["outcome"]["status"], "committed",
            "{result}"
        );
        let receipt = &result["result"]["outcome"]["receipt"];
        review.finish();
        for _ in 0..2 {
            let output = Command::new(&fixture.host)
                .args(["completion-fixture-readback", "--startup"])
                .arg(&fixture.startup)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(value["standing"], "fixture_owner_readback_only");
            assert_eq!(&value["receipt"], receipt);
            assert_eq!(fixture.entries(), 1);
        }
        let denied = Command::new(&fixture.host)
            .args(["completion-serve", "--startup"])
            .arg(&fixture.startup)
            .output()
            .unwrap();
        assert!(!denied.status.success());
        assert!(
            String::from_utf8_lossy(&denied.stderr)
                .contains("test build cannot serve production completion")
        );
    }
}
