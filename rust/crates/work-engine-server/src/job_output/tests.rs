use super::validate;
use serde_json::{Value, json};

fn sample() -> (Value, Value) {
    let request = "11111111-1111-4111-8111-111111111111";
    let digest = "b".repeat(64);
    let receipt = "c".repeat(64);
    let arguments = json!({"request_id":request,"authorization_id":"fixture"});
    let operation = json!({
        "schema":"work-engine.coding-job.v1","operation_id":"a".repeat(64),
        "request_id":request,"authorization_id":"fixture","job_id":"fixture",
        "state_dir":"/private/state","job_sha256":"d".repeat(64),
        "expected_host_input_digest":digest,"host_input_digest":digest,
        "state":"collected","entered_possible":true,"collector_live":false,
        "host_phase":"collected","host_removed":true,"host_receipt_id":receipt,
        "collector_run":{},"collector_reconcile":null,
        "process_cleanup":{"run":"owned_process_group_collected","reconcile":null},
        "limitations":[],"settlement":"not_established_by_submission"
    });
    let output = json!({
        "schema":"work-engine.coding-job.v1","result":"available","operation":operation,
        "terminal":{"status":"component_passed","phase":"collected"},
        "validation":{"rpc_verified":true,"failures":[],"coding_completed":true},
        "cleanup":{"removed":true,"daemon_exit":0,"tool_removed":true,
            "collector":{"run":"owned_process_group_collected","reconcile":null}},
        "acceptance":"owner_pending","domain_effect_settlement":"not_claimed",
        "receipt_id":receipt,"input_digest":digest,
        "receipt":{"job_id":"fixture","input_digest":digest},
        "host_output":{"action":"run","exit_code":0,"note":"verbatim Host stdout"}
    });
    (arguments, output)
}

#[test]
fn nested_projections_reject_missing_unknown_and_malformed_fields() {
    let (args, good) = sample();
    assert!(validate("read_coding_job_result", &args, &good).is_ok());
    for section in ["terminal", "validation", "cleanup"] {
        let mut bad = good.clone();
        bad[section] = json!({});
        assert!(
            validate("read_coding_job_result", &args, &bad).is_err(),
            "{section}"
        );
        let mut bad = good.clone();
        bad[section]["unexpected"] = json!(true);
        assert!(
            validate("read_coding_job_result", &args, &bad).is_err(),
            "{section}"
        );
        for field in good[section].as_object().unwrap().keys() {
            let mut bad = good.clone();
            bad[section].as_object_mut().unwrap().remove(field);
            assert!(
                validate("read_coding_job_result", &args, &bad).is_err(),
                "{section}.{field}"
            );
        }
    }
    for (section, field, value) in [
        ("terminal", "status", json!("accepted")),
        ("terminal", "phase", json!(false)),
        ("validation", "rpc_verified", json!({})),
        ("validation", "failures", json!([false])),
        ("validation", "coding_completed", json!(null)),
        ("cleanup", "removed", json!("yes")),
        ("cleanup", "daemon_exit", json!(0.5)),
        ("cleanup", "tool_removed", json!([])),
        (
            "cleanup",
            "collector",
            json!({"run":"settled","reconcile":null}),
        ),
        ("host_output", "exit_code", json!("success")),
    ] {
        let mut bad = good.clone();
        bad[section][field] = value;
        assert!(
            validate("read_coding_job_result", &args, &bad).is_err(),
            "{section}.{field}"
        );
    }
}

#[test]
fn required_nullable_fields_cannot_be_omitted() {
    let (args, good) = sample();
    for field in [
        "host_input_digest",
        "host_phase",
        "host_removed",
        "host_receipt_id",
        "collector_run",
        "collector_reconcile",
    ] {
        let mut bad = good["operation"].clone();
        bad.as_object_mut().unwrap().remove(field);
        assert!(
            validate("read_coding_job_operation", &args, &bad).is_err(),
            "{field}"
        );
    }
    for location in ["process_cleanup", "collector"] {
        for field in ["run", "reconcile"] {
            let mut bad = good.clone();
            let value = if location == "process_cleanup" {
                &mut bad["operation"][location]
            } else {
                &mut bad["cleanup"][location]
            };
            value.as_object_mut().unwrap().remove(field);
            assert!(
                validate("read_coding_job_result", &args, &bad).is_err(),
                "{location}.{field}"
            );
        }
    }
}

#[test]
fn explicit_null_and_not_established_keep_their_distinct_meaning() {
    let (args, mut good) = sample();
    good["terminal"] = json!({"status":"not_established","phase":"not_established"});
    good["validation"] = json!({"rpc_verified":"not_established","failures":"not_established","coding_completed":"not_established"});
    good["cleanup"] = json!({"removed":"not_established","daemon_exit":null,"tool_removed":"not_established","collector":{"run":null,"reconcile":null}});
    for exit in [json!(null), json!("not_established"), json!(137)] {
        good["host_output"]["exit_code"] = exit;
        assert!(validate("read_coding_job_result", &args, &good).is_ok());
    }
}

#[test]
#[ignore = "requires CLI_JOB_QUALIFICATION_RESULTS, a JSON list of saved response fixture paths"]
fn recorded_offline_responses_remain_compatible() {
    let paths: Vec<String> = serde_json::from_str(
        &std::env::var("CLI_JOB_QUALIFICATION_RESULTS").expect("fixture paths required"),
    )
    .unwrap();
    let mut checked = 0;
    for path in paths {
        let value: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let args = json!({"request_id":value["request_id"],"authorization_id":format!("offline-v2-{}",value["case"].as_str().unwrap())});
        for call in value["calls"].as_array().unwrap() {
            let owner = &call["response"]["envelope"]["owner_result"];
            if owner.is_null() {
                continue;
            }
            validate(call["operation"].as_str().unwrap(), &args, owner).unwrap();
            checked += 1;
        }
    }
    assert!(checked > 5);
    println!("validated {checked} recorded owner responses");
}
