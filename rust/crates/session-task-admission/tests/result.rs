use session_task_admission::{
    Ref,
    result::{Accounting, Clock, Custody, Disposition, unresolved},
};
#[test]
fn missing_contract_never_infers_success_or_closed_custody() {
    let task = Ref {
        id: "task".into(),
        revision: "r1".into(),
        sha256: "0".repeat(64),
    };
    for present in [false, true] {
        let result = unresolved(
            &task,
            "a",
            Some("s"),
            Some("t"),
            present,
            Clock {
                first_funded_at: None,
                admitted_at: None,
                observed_start_at: None,
                close_by: "deadline",
            },
            Accounting {
                units: "usd",
                limit: "1",
                reservation: None,
                observed_usage: None,
                coverage: "unknown",
                unsettled: None,
            },
            Custody {
                native: Disposition {
                    id: Some("s"),
                    disposition: "unknown",
                },
                children: vec![],
                processes: vec![],
            },
            "now",
            "r1",
        );
        let json = serde_json::to_value(result).unwrap();
        assert!(json["outcome"].is_null());
        assert_eq!(
            json["operational_status"],
            "missing_or_invalid_worker_contract"
        );
        assert_eq!(json["custody"]["native"]["disposition"], "unknown");
        assert!(json["accounting"]["observed_usage"].is_null());
    }
}
