//! Supplied-payload coherence only. Monetary expectations remain oracle owned.
use pi_accounting_spec_check::{
    resolve_accounting_payloads, validate_accounting_bundle_with_payloads,
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::PathBuf};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn load(path: &str) -> (Value, BTreeMap<String, Value>) {
    let path = root().join(path);
    let b = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let ps = resolve_accounting_payloads(&root(), &path, &b).unwrap();
    (b, ps)
}
fn equal() -> (Value, BTreeMap<String, Value>) {
    load("oracle/r9-pilot/cases/equal-reservations.bundle.json")
}
fn wrong(f: impl FnOnce(&mut Value, &mut BTreeMap<String, Value>), needle: &str) {
    let (mut b, mut p) = equal();
    f(&mut b, &mut p);
    let e = validate_accounting_bundle_with_payloads(&b, &p).unwrap_err();
    assert!(e.contains(needle), "{e} does not contain {needle}");
}
#[test]
fn referenced_subject_scope_fragment_and_inventory_are_exact() {
    let (b, p) = equal();
    validate_accounting_bundle_with_payloads(&b, &p).unwrap();
    wrong(
        |b, _| b["documents"][0]["grants"][0]["covered_extent_ref"] = json!("sha256:stale"),
        "covered extent",
    );
    wrong(
        |b, _| b["documents"][0]["initial_contracts"][0]["scope_id"] = json!("wrong-scope"),
        "initial contract/scope/namespace",
    );
    wrong(
        |b, _| {
            b["documents"][0]["initial_contracts"][0]["reference_namespace"] = json!("admission")
        },
        "initial contract/scope/namespace",
    );
    wrong(|_, p| p.clear(), "unresolved exact payload");
    wrong(
        |b, p| {
            let old = b["documents"][0]["grants"][0]["exact_payload_ref"]
                .as_str()
                .unwrap()
                .to_owned();
            let v = p.remove(&old).unwrap();
            let new = old.replace("#S1/turn-1", "#S1/turn-9");
            p.insert(new.clone(), v);
            b["documents"][0]["grants"][0]["exact_payload_ref"] = json!(new);
        },
        "source fragment",
    );
    wrong(
        |_, p| {
            let v = p.values_mut().next().unwrap();
            v["raw_input_sha256"] = json!("stale");
        },
        "covered extent",
    );
    wrong(
        |_, p| {
            p.values_mut().next().unwrap()["grant_payloads"][0]["exact_origin_refs"] =
                json!(["S1/turn-1:1"]);
        },
        "covered origin inventory",
    );
    wrong(
        |_, p| {
            p.values_mut().next().unwrap()["case_id"] = json!("other-case");
        },
        "payload subject",
    );
}
#[test]
fn conditional_pair_binds_endpoint_bytes_not_only_ids() {
    let (b, p) = load("oracle/r8-controls/VC07-conditional.bundle.json");
    validate_accounting_bundle_with_payloads(&b, &p).unwrap();
    let mut bad = b.clone();
    bad["documents"][0]["origins"][0]["raw_bytes_sha256"] =
        json!("0000000000000000000000000000000000000000000000000000000000000000");
    assert!(
        validate_accounting_bundle_with_payloads(&bad, &p)
            .unwrap_err()
            .contains("endpoint bytes")
    );
    let mut bad = b.clone();
    bad["documents"][0]["origins"][0]["original_position"] = json!(2);
    assert!(validate_accounting_bundle_with_payloads(&bad, &p).is_err());
    let mut bad = p.clone();
    bad.values_mut().next().unwrap()["conditional_harness_assumptions"]["exact_row_coreference"]
        ["from"] = json!("u");
    assert!(
        validate_accounting_bundle_with_payloads(&b, &bad)
            .unwrap_err()
            .contains("pair mismatch")
    );
    let mut bad = p;
    bad.values_mut().next().unwrap()["conditional_harness_assumptions"]["exact_row_coreference"]
        ["source_namespace"] = json!("other-scope");
    assert!(
        validate_accounting_bundle_with_payloads(&b, &bad)
            .unwrap_err()
            .contains("namespace/extent")
    );
}
#[test]
fn resolution_rejects_missing_ambiguous_or_unknown_payloads() {
    let (mut b, p) = equal();
    b["documents"][0]["grants"][0]["exact_payload_ref"] = json!("missing-file.json#S1/turn-1");
    assert!(
        resolve_accounting_payloads(
            &root(),
            &root().join("oracle/r9-pilot/cases/anything.json"),
            &b
        )
        .is_err()
    );
    assert!(validate_accounting_bundle_with_payloads(&b, &p).is_err());
    let (mut b, mut p) = equal();
    b["documents"][0]["grants"][0]["guarantee_kind"] = json!("continuity");
    // This grant is still referenced as initial scope, so the base structural check rejects first.
    assert!(validate_accounting_bundle_with_payloads(&b, &p).is_err());
    p.clear();
}
#[test]
fn export_payload_cli_subjects_when_requested() {
    let Ok(dest) = std::env::var("R10_SPEC_FIXTURE_OUT") else {
        return;
    };
    let dest = PathBuf::from(dest);
    std::fs::create_dir_all(&dest).unwrap();
    let (b, p) = equal();
    validate_accounting_bundle_with_payloads(&b, &p).unwrap();
    std::fs::write(
        dest.join("positive.json"),
        serde_json::to_vec_pretty(&b).unwrap(),
    )
    .unwrap();
    for (label, key, value) in [
        ("stale-extent", "covered_extent_ref", json!("sha256:stale")),
        (
            "wrong-fragment",
            "exact_payload_ref",
            json!("oracle/r9-pilot/cases/equal-reservations.provenance.json#S1/turn-9"),
        ),
    ] {
        let mut bad = b.clone();
        bad["documents"][0]["grants"][0][key] = value;
        std::fs::write(
            dest.join(format!("{label}.json")),
            serde_json::to_vec_pretty(&bad).unwrap(),
        )
        .unwrap();
    }
    let mut bad = b;
    bad["documents"][0]["initial_contracts"][0]["scope_id"] = json!("other-scope");
    std::fs::write(
        dest.join("wrong-scope.json"),
        serde_json::to_vec_pretty(&bad).unwrap(),
    )
    .unwrap();
}

#[test]
fn all_selected_supplied_payloads_are_coherent() {
 for path in [
 "oracle/r8-completion-B/P9-claimed-charge-above-reservation.bundle.json",
 "oracle/r8-completion-B/VC11-multi-turn-session.bundle.json",
 "oracle/r8-completion-A/P132-FG-VC11-gated.bundle.json",
 "oracle/r8-completion-B/VC07-read-failure-paired.bundle.json",
 "oracle/r9-pilot/cases/model7-VC01-openai-usage-settlement.bundle.json",
 "oracle/r9-pilot/cases/model7-A04-undated-settlement-between-dated-neighbours.bundle.json",
 "oracle/r9-pilot/cases/equal-reservations.bundle.json",
 "oracle/r8-controls/VC07-conditional.bundle.json",
 "oracle/r8-controls/VC07-unlinked.bundle.json",
 ] {let (b,p)=load(path);validate_accounting_bundle_with_payloads(&b,&p).unwrap_or_else(|e|panic!("{path}: {e}"));}
}
#[test]
fn gated_derivation_fragment_and_endpoint_inventory_are_exact() {
 let (b,p)=load("oracle/r8-completion-A/P132-FG-VC11-gated.bundle.json");
 validate_accounting_bundle_with_payloads(&b,&p).unwrap();
 let mut bad=b.clone();bad["documents"][0]["origins"][0]["raw_bytes_sha256"]=json!("0000000000000000000000000000000000000000000000000000000000000000");
 assert!(validate_accounting_bundle_with_payloads(&bad,&p).unwrap_err().contains("origin bytes"));
 let mut bad=b.clone();bad["documents"][0]["initial_contracts"][0]["scope_id"]=json!("wrong-scope");
 assert!(validate_accounting_bundle_with_payloads(&bad,&p).unwrap_err().contains("contract/scope/namespace"));
 let mut bad=b;let r=bad["documents"][0]["grants"][0]["exact_payload_ref"].as_str().unwrap().to_owned();let new=r.replace("scope:S1/turn-1","scope:S1/turn-2");bad["documents"][0]["grants"][0]["exact_payload_ref"]=json!(new.clone());let mut p=p;p.insert(new,p[&r].clone());
 assert!(validate_accounting_bundle_with_payloads(&bad,&p).unwrap_err().contains("derivation payload fragment"));
}
