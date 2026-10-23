//! E2 review fix P3-3: serde schema errors carry the failing path in `at`.

mod support;
use proposal_evaluation::scan::locate_data_error;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use support::*;

fn derive_to(w: &World, name: &str) -> PathBuf {
    let out = w.dir.join(name);
    let run = cli(&[
        "compare".to_string(),
        "--contract".to_string(),
        s(&w.contract),
        "--estimates".to_string(),
        s(&w.sets[0]),
        "--estimates".to_string(),
        s(&w.sets[1]),
        "--derived-at".to_string(),
        "2026-10-23T00:00:00Z".to_string(),
        "--out".to_string(),
        s(&out),
    ]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    out
}

fn read(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

// ---- serde schema errors carry the failing path (P3-3) ----

/// The `at` of the one `schema` error `check_comparison_bytes` reports for `doc`.
fn finding_schema_at(doc: &Value) -> String {
    let bytes = serde_json::to_vec_pretty(doc).unwrap();
    let report = proposal_evaluation::check_comparison_bytes(&bytes, false).unwrap();
    assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
    assert_eq!(report.errors[0].code, "schema", "{:?}", report.errors);
    report.errors[0].at.clone()
}

#[test]
fn a_contract_with_a_float_version_is_located_at_version() {
    let mut c = valid_contract();
    c["version"] = json!(1.0);
    let bytes = serde_json::to_vec_pretty(&c).unwrap();
    let report = proposal_evaluation::check_contract_bytes(&bytes).unwrap();
    assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
    assert_eq!(report.errors[0].code, "schema");
    assert_eq!(report.errors[0].at, "version");
    // and through compare, which names the contract it refused
    let w = world("ptr-float-version");
    rewrite_contract(&w, |c| c["version"] = json!(1.0));
    let run = cli(&[
        "compare".to_string(),
        "--contract".to_string(),
        s(&w.contract),
        "--estimates".to_string(),
        s(&w.sets[0]),
        "--estimates".to_string(),
        s(&w.sets[1]),
        "--derived-at".to_string(),
        "2026-10-23T00:00:00Z".to_string(),
    ]);
    assert_eq!(run.code, 3, "{}", run.stderr);
    assert!(
        has(&run.pairs(), "schema", "contract.version"),
        "{:?}",
        run.pairs()
    );
}

#[test]
fn a_finding_schema_error_is_located_at_the_key_or_value() {
    let w = world("ptr-finding-schema");
    let base = read(&derive_to(&w, "finding.json"));

    let mut d = base.clone();
    d["weights"] = json!({"cost": 1});
    assert_eq!(finding_schema_at(&d), "weights");

    let mut d = base.clone();
    d["pairs"][0]["extra"] = json!(true);
    assert_eq!(finding_schema_at(&d), "pairs.0.extra");

    let mut d = base.clone();
    d["pairs"][0]["measures"][0]["relation"] = json!("sideways");
    assert_eq!(finding_schema_at(&d), "pairs.0.measures.0.relation");

    let mut d = base.clone();
    d["derived_at"] = json!(7);
    assert_eq!(finding_schema_at(&d), "derived_at");

    // a missing field is placed at the field, under the object that lacks it
    let mut d = base.clone();
    d.as_object_mut().unwrap().remove("frontier");
    assert_eq!(finding_schema_at(&d), "frontier");

    let mut d = base.clone();
    d["pairs"][0].as_object_mut().unwrap().remove("outcome");
    assert_eq!(finding_schema_at(&d), "pairs.0.outcome");
}

#[test]
fn an_estimate_set_schema_error_is_located() {
    let mut set = dummy_set();
    set["estimates"][1]["competing"][0]["value"] = json!(true);
    let bytes = serde_json::to_vec_pretty(&set).unwrap();
    let report = proposal_evaluation::check_estimate_set_bytes(&bytes).unwrap();
    assert_eq!(report.errors.len(), 1, "{:?}", report.errors);
    assert_eq!(report.errors[0].code, "schema");
    assert_eq!(report.errors[0].at, "estimates.1");
}

#[test]
fn locating_works_on_compact_documents_and_escaped_or_non_ascii_keys() {
    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct Inner {
        n: u8,
    }
    #[derive(Debug, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    #[allow(dead_code)]
    struct Outer {
        inner: Vec<Inner>,
    }
    let at = |text: &str| {
        let e = serde_json::from_str::<Outer>(text).unwrap_err();
        locate_data_error(text.as_bytes(), &e)
    };
    assert_eq!(at(r#"{"inner":[{"n":1},{"n":"x"}]}"#), "inner.1.n");
    assert_eq!(at(r#"{"inner":[{"n":1,"ünï":2}]}"#), "inner.0.ünï");
    assert_eq!(at(r#"{"inner":[{"n":1,"a\"b":2}]}"#), "inner.0.a\"b");
    assert_eq!(at(r#"{"inner":[{}]}"#), "inner.0.n");
    assert_eq!(at(r#"{}"#), "inner");
    assert_eq!(
        at("{\n  \"inner\": [\n    {\"n\": 300}\n  ]\n}"),
        "inner.0.n"
    );
    // a document that is not an object at the root has the root path `$`
    let e = serde_json::from_str::<Outer>("5").unwrap_err();
    assert_eq!(locate_data_error(b"5", &e), "$");
}
