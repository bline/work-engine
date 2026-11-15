//! Independent R8 controls. No canonical financial helper is imported.
//! This runs the actual Python entry points and checks finite raw-derived lows.
//! It does not execute a raw financial reducer or establish full corpus conformance.
use serde_json::Value;
use std::{fs, path::PathBuf, process::Command};
fn root() -> PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..") }
#[test]
fn actual_python_entrypoints_and_finite_literals() {
    for file in ["test_r8_entrypoints.py", "test_r8_assertions.py"] {
        let status = Command::new("python3").arg(root().join("validator").join(file))
            .current_dir(root()).status().expect("actual Python test process");
        assert!(status.success(), "actual entry-point control failed: {file}");
    }
}
#[test]
fn raw_supported_local_lows_survive_invalid_settlement() {
    // USD-per-million rates yield nano-per-token factors: Astra input10000,
    // output50000; Sonnet input2000; Terra output5000. These calculations are
    // independently authored and do not call canonical valuation logic.
    for (id, expected, raw_in, raw_out) in [
        ("A06-reservation-usage-null-status-no-id", 1000i128*10000+100*50000,1000,100),
        ("P133-A09-conflicting-reservations-with-settlement",1000*10000+100*50000,1000,100),
        ("P130-R05-conflicting-keeps-valuation-high",10000*2000,10000,0),
        ("V01-cheaper-response-model-undercharge",12000*5000,0,12000),
    ] {
        let c:Value=serde_json::from_slice(&fs::read(root().join("corpus/cases").join(format!("{id}.json"))).unwrap()).unwrap();
        let attempt=&c["expected"]["attempts"][0];
        assert_eq!(attempt["exposure"]["low"]["nano"],expected.to_string(),"{id}");
        assert_eq!(attempt["exposure"]["low"]["basis"],"usage_valuation");
        let rows=c["inputs"]["sessions"][0]["logs"]["turn-1"]["lines"].as_array().unwrap();
        let requests:Vec<Value>=rows.iter().filter_map(|line|serde_json::from_str::<Value>(line.as_str()?).ok()).filter(|row|row["event"]=="request").collect();
        assert!(!requests.is_empty());
        assert_eq!(requests[0]["usage"]["input_tokens"],raw_in);
        assert_eq!(requests[0]["usage"]["output_tokens"],raw_out);
        assert!(attempt["settlement"]["decision"]=="rejected"||attempt["settlement"]["decision"]=="conflicting");
    }
}
