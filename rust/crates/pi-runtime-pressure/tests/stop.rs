use pi_runtime_pressure::stop_cases::{case_name, classify, release_due};
use serde_json::json;
fn base() -> (serde_json::Value, serde_json::Value, serde_json::Value) {
    (
        json!({"phase":"collected","failures":["cancellation requested"],"processes":[{"timed_out":false}],"removed":true,"job":{"deadline_unix_seconds":100},"coding":{"tool_removed":true,"final_acceptance":null}}),
        json!({"receipt":{"status":"not_established"}}),
        json!({"observed_ms":80000,"cancel_requested_ms":80001,"cancel_ack":{"status":"cancellation_requested"},"cancel_tool_age_seconds":12.0,"tool_process_index":0,"native_request_sha256":"x".repeat(64),"native_message_sha256":"y".repeat(64),"active_tool_stdout_sha256":"z".repeat(64)}),
    )
}
#[test]
fn completed_missing_and_late_are_not_cancellation() {
    let (j, mut r, mut w) = base();
    assert!(classify("cancel", &j, &r, &w).is_ok());
    r["receipt"]["status"] = json!("component_passed");
    assert!(classify("cancel", &j, &r, &w).is_err());
    r["receipt"]["status"] = json!("not_established");
    w["observed_ms"] = json!(100000);
    assert!(classify("cancel", &j, &r, &w).is_err());
    w["observed_ms"] = json!(80000);
    w["native_request_sha256"] = json!("");
    assert!(classify("cancel", &j, &r, &w).is_err());
}
#[test]
fn deadline_must_precede_command_timeout_and_have_no_cancel() {
    let (j, r, mut w) = base();
    w["cancel_requested_ms"] = serde_json::Value::Null;
    w["tool_finished_observed_ms"] = json!(100001);
    w["tool_exit_age_seconds"] = json!(24.0);
    assert!(classify("deadline", &j, &r, &w).is_ok());
    w["tool_exit_age_seconds"] = json!(30.0);
    assert!(classify("deadline", &j, &r, &w).is_err());
    w["tool_exit_age_seconds"] = json!(24.0);
    w["tool_finished_observed_ms"] = json!(99999);
    assert!(classify("deadline", &j, &r, &w).is_err());
}
#[test]
fn fixed_modes_and_release_window() {
    assert_eq!(case_name(Some("--cancel-after-pressure")), Some("cancel"));
    assert_eq!(
        case_name(Some("--deadline-after-pressure")),
        Some("deadline")
    );
    assert_eq!(case_name(Some("other")), None);
    assert!(!release_due("deadline", 100, 74999));
    assert!(release_due("deadline", 100, 75000));
    assert!(release_due("cancel", 100, 1));
}

#[test]
fn cancellation_must_be_persisted_after_activity_and_before_deadline() {
    let (mut j, r, mut w) = base();
    assert!(classify("cancel", &j, &r, &w).is_ok());
    for time in [79999, 100000, 100001] {
        w["cancel_requested_ms"] = json!(time);
        assert!(classify("cancel", &j, &r, &w).is_err(), "{time}");
    }
    w["cancel_requested_ms"] = json!(99999);
    assert!(classify("cancel", &j, &r, &w).is_ok());
    j["failures"] = json!(["process deadline/output/input failure"]);
    assert!(classify("cancel", &j, &r, &w).is_err());
    j["failures"] = json!(["cancellation requested"]);
    w["cancel_ack"] = serde_json::Value::Null;
    assert!(classify("cancel", &j, &r, &w).is_err());
}
