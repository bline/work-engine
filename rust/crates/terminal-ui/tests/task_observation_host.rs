use std::{path::PathBuf, time::Duration};
use work_engine_terminal_ui::task_observation::{self, Component, ReadError, Request};

/// Invoked only by the installed-host proof, with a pinned executable and endpoint.
#[test]
#[ignore = "requires the lifecycle-proof owned installed host fixture"]
fn installed_host_read_only_consumer() {
    let endpoint = PathBuf::from(std::env::var_os("S8_UI_ENDPOINT").expect("endpoint"));
    let request: Request =
        serde_json::from_str(&std::env::var("S8_UI_REQUEST").expect("request")).unwrap();
    let case = std::env::var("S8_UI_CASE").expect("case");
    let result = task_observation::read(&endpoint, &request, Duration::from_secs(5));
    if case == "transport_unavailable" {
        assert!(matches!(
            result,
            Err(ReadError::Unavailable | ReadError::Deadline)
        ));
        println!("TO2 consumer: unavailable transport preserved");
        return;
    }
    let observed = result.unwrap();
    let expected_identity: serde_json::Value =
        serde_json::from_str(&std::env::var("S8_UI_IDENTITY").expect("identity")).unwrap();
    assert_eq!(
        serde_json::to_value(&observed.host_identity).unwrap(),
        expected_identity
    );
    let owner = &observed.components["owner"];
    match case.as_str() {
        "absent" => {
            assert!(matches!(owner,Component::Absent{record_kind,..} if record_kind=="task"));
            assert!(matches!(
                observed.components["native_delivery_status"],
                Component::NotApplicable { .. }
            ));
        }
        "empty" => {
            assert!(owner.value().is_some());
            assert_eq!(
                observed.components["native_delivery_status"]
                    .value()
                    .unwrap()["segments"],
                serde_json::json!([])
            );
        }
        "segment" => {
            assert!(owner.value().is_some());
            assert!(
                !observed.components["native_delivery_status"]
                    .value()
                    .unwrap()["segments"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        }
        "question" => {
            assert_eq!(owner.value().unwrap()["questions"][0]["state"], "pending");
        }
        "result" => {
            assert!(owner.value().unwrap()["result"].is_object());
        }
        _ => panic!("unrecognized proof case"),
    }
    println!(
        "TO2 consumer: {}",
        serde_json::to_string(&observed).unwrap()
    );
}
