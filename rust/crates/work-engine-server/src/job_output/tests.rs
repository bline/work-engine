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

// Serde derive accepts a positional array for a struct and a one-key object for a unit enum
// variant; the JSON schema accepts neither. These tables pin the schema's representations.
fn positional(value: &Value, order: &[&str]) -> Value {
    Value::Array(order.iter().map(|k| value[*k].clone()).collect())
}

const OPERATION_ORDER: &[&str] = &[
    "schema",
    "operation_id",
    "request_id",
    "authorization_id",
    "job_id",
    "state_dir",
    "job_sha256",
    "expected_host_input_digest",
    "host_input_digest",
    "state",
    "entered_possible",
    "collector_live",
    "host_phase",
    "host_removed",
    "host_receipt_id",
    "collector_run",
    "collector_reconcile",
    "process_cleanup",
    "limitations",
    "settlement",
    "entry",
];
const CLEANUP_ORDER: &[&str] = &["run", "reconcile"];

fn operation_array(operation: &Value) -> Value {
    let mut operation = operation.clone();
    operation["process_cleanup"] = positional(&operation["process_cleanup"], CLEANUP_ORDER);
    positional(&operation, OPERATION_ORDER)
}

fn as_variant(name: &str) -> Value {
    json!({ name: null })
}

fn accepted(operation: &str, args: &Value, cases: Vec<(String, Value)>) -> Vec<String> {
    cases
        .into_iter()
        .filter(|(_, value)| validate(operation, args, value).is_ok())
        .map(|(label, _)| label)
        .collect()
}

#[test]
fn struct_positional_arrays_are_rejected_for_every_adapter_object() {
    let (args, good) = sample();
    let mut cases: Vec<(String, Value)> = Vec::new();
    let mut set = |label: &str, f: &dyn Fn(&mut Value)| {
        let mut bad = good.clone();
        f(&mut bad);
        cases.push((label.to_string(), bad));
    };
    set("terminal", &|b| {
        b["terminal"] = json!(["component_passed", "collected"]);
    });
    set("validation", &|b| {
        b["validation"] = json!([true, [], true]);
    });
    set("cleanup", &|b| {
        b["cleanup"] = json!([
            true,
            0,
            true,
            {"run":"owned_process_group_collected","reconcile":null}
        ]);
    });
    set("cleanup.collector", &|b| {
        b["cleanup"]["collector"] = json!(["owned_process_group_collected", null]);
    });
    set("host_output", &|b| {
        b["host_output"] = json!(["run", 0, "verbatim Host stdout"]);
    });
    set("operation.process_cleanup", &|b| {
        b["operation"]["process_cleanup"] = json!(["owned_process_group_collected", null]);
    });
    set("operation", &|b| {
        let operation = b["operation"].clone();
        b["operation"] = operation_array(&operation);
    });
    let mut bad = good.clone();
    bad["operation"] = operation_array(&good["operation"]);
    let wrong = accepted("read_coding_job_result", &args, cases);
    assert!(wrong.is_empty(), "positional arrays accepted: {wrong:?}");

    let operation = operation_array(&good["operation"]);
    assert!(validate("read_coding_job_operation", &args, &operation).is_err());
    let mut submit = good["operation"].clone();
    submit["entry"] = json!("collector_started");
    assert!(validate("submit_coding_job", &args, &submit).is_ok());
    assert!(validate("submit_coding_job", &args, &operation_array(&submit)).is_err());
    let cancel = json!({
        "schema":"work-engine.coding-job.v1","operation_id":"a".repeat(64),
        "request_id":args["request_id"],"cancellation":"requested",
        "cancellation_established":false,"container_closure_established":false,
        "settlement":"not_claimed","next":"read_coding_job_operation"
    });
    assert!(validate("cancel_coding_job", &args, &cancel).is_ok());
    let array = positional(
        &cancel,
        &[
            "schema",
            "operation_id",
            "request_id",
            "cancellation",
            "cancellation_established",
            "container_closure_established",
            "settlement",
            "next",
        ],
    );
    assert!(validate("cancel_coding_job", &args, &array).is_err());
    let unavailable = json!({
        "schema":"work-engine.coding-job.v1","result":"not_available","operation":good["operation"]
    });
    assert!(validate("read_coding_job_result", &args, &unavailable).is_ok());
    for value in [
        json!([
            "work-engine.coding-job.v1",
            "not_available",
            good["operation"]
        ]),
        json!([
            "work-engine.coding-job.v1",
            "not_available",
            operation_array(&good["operation"])
        ]),
    ] {
        assert!(validate("read_coding_job_result", &args, &value).is_err());
    }
    assert!(validate("read_coding_job_result", &args, &bad).is_err());
}

#[test]
fn enum_one_key_objects_are_rejected_for_every_adapter_enum() {
    let (args, good) = sample();
    type Edit = fn(&mut Value);
    let result_cases: Vec<(&str, Edit)> = vec![
        ("terminal.status", |b| {
            b["terminal"]["status"] = as_variant("component_passed")
        }),
        ("terminal.phase", |b| {
            b["terminal"]["phase"] = as_variant("collected")
        }),
        ("terminal.status sentinel", |b| {
            b["terminal"]["status"] = as_variant("not_established")
        }),
        ("terminal.phase sentinel", |b| {
            b["terminal"]["phase"] = as_variant("not_established")
        }),
        ("validation.rpc_verified sentinel", |b| {
            b["validation"]["rpc_verified"] = as_variant("not_established")
        }),
        ("validation.failures sentinel", |b| {
            b["validation"]["failures"] = as_variant("not_established")
        }),
        ("validation.coding_completed sentinel", |b| {
            b["validation"]["coding_completed"] = as_variant("not_established")
        }),
        ("cleanup.removed sentinel", |b| {
            b["cleanup"]["removed"] = as_variant("not_established")
        }),
        ("cleanup.daemon_exit sentinel", |b| {
            b["cleanup"]["daemon_exit"] = as_variant("not_established")
        }),
        ("cleanup.tool_removed sentinel", |b| {
            b["cleanup"]["tool_removed"] = as_variant("not_established")
        }),
        ("cleanup.collector.run", |b| {
            b["cleanup"]["collector"]["run"] = as_variant("owned_process_group_collected")
        }),
        ("cleanup.collector.reconcile", |b| {
            b["cleanup"]["collector"]["reconcile"] = as_variant("not_established")
        }),
        ("host_output.action", |b| {
            b["host_output"]["action"] = as_variant("run")
        }),
        ("host_output.exit_code sentinel", |b| {
            b["host_output"]["exit_code"] = as_variant("not_established")
        }),
        ("operation.state", |b| {
            b["operation"]["state"] = as_variant("collected")
        }),
        ("operation.host_phase", |b| {
            b["operation"]["host_phase"] = as_variant("collected")
        }),
        ("operation.host_phase sentinel", |b| {
            b["operation"]["host_phase"] = as_variant("not_established")
        }),
        ("operation.settlement", |b| {
            b["operation"]["settlement"] = as_variant("not_established_by_submission")
        }),
        ("operation.process_cleanup.run", |b| {
            b["operation"]["process_cleanup"]["run"] = as_variant("owned_process_group_collected")
        }),
        ("operation.process_cleanup.reconcile", |b| {
            b["operation"]["process_cleanup"]["reconcile"] = as_variant("not_established")
        }),
        ("result", |b| b["result"] = as_variant("available")),
    ];
    let cases = result_cases
        .into_iter()
        .map(|(label, edit)| {
            let mut bad = good.clone();
            edit(&mut bad);
            (label.to_string(), bad)
        })
        .collect();
    let wrong = accepted("read_coding_job_result", &args, cases);
    assert!(wrong.is_empty(), "one-key enum objects accepted: {wrong:?}");

    let mut submit = good["operation"].clone();
    submit["entry"] = json!("existing_operation");
    assert!(validate("submit_coding_job", &args, &submit).is_ok());
    submit["entry"] = as_variant("existing_operation");
    assert!(validate("submit_coding_job", &args, &submit).is_err());
    // `entry` is absent outside submission and is never null.
    let mut submit = good["operation"].clone();
    submit["entry"] = json!(null);
    assert!(validate("submit_coding_job", &args, &submit).is_err());
    assert!(validate("read_coding_job_operation", &args, &submit).is_err());

    let mut cancel = json!({
        "schema":"work-engine.coding-job.v1","operation_id":"a".repeat(64),
        "request_id":args["request_id"],"cancellation":"request_not_recorded",
        "cancellation_established":false,"container_closure_established":false,
        "settlement":"not_claimed","next":"read_coding_job_operation"
    });
    assert!(validate("cancel_coding_job", &args, &cancel).is_ok());
    for (field, name) in [("cancellation", "requested"), ("settlement", "not_claimed")] {
        let original = cancel[field].clone();
        cancel[field] = as_variant(name);
        assert!(
            validate("cancel_coding_job", &args, &cancel).is_err(),
            "{field}"
        );
        cancel[field] = original;
    }
}

#[test]
fn string_enum_and_null_controls_stay_accepted_and_unknown_fields_rejected() {
    let (args, good) = sample();
    let mut ok = good.clone();
    ok["operation"]["host_phase"] = json!(null);
    ok["cleanup"]["collector"]["reconcile"] = json!("not_established");
    ok["host_output"]["exit_code"] = json!(null);
    ok["terminal"] = json!({"status":"component_failed","phase":"create_unknown"});
    assert!(validate("read_coding_job_result", &args, &ok).is_ok());
    for (section, path) in [
        ("operation", "operation"),
        ("terminal", "terminal"),
        ("host_output", "host_output"),
    ] {
        let mut bad = good.clone();
        bad[path]["extra"] = json!(1);
        assert!(
            validate("read_coding_job_result", &args, &bad).is_err(),
            "{section}"
        );
    }
    let mut bad = good.clone();
    bad["terminal"]["status"] = json!("COMPONENT_PASSED");
    assert!(validate("read_coding_job_result", &args, &bad).is_err());
}
