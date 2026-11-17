//! C-validity sole ownership (R8-OWNER-01) and exact numeric/string domain agreement
//! (R8-SCHEMA-DOMAIN-01). Structural declarations only; no financial evaluation.
//! Each negative asserts the predicate named in its diagnostic; each is paired with a positive
//! built from the same mutation that must pass, so a blanket rejection cannot satisfy it.
use pi_accounting_spec_check::*;
use serde_json::{Value, json};
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn spec() -> Spec {
    read_spec(&root()).unwrap()
}
fn decision(variant: &str) -> Constructor {
    let fields: &[&str] = match variant {
        "absent" | "not_applicable" => &[],
        "valid" => &["basis", "charged", "settlement"],
        "rejected" => &["claimed", "reasons"],
        "conflicting" => &["claimed", "settlements"],
        other => panic!("unknown variant {other}"),
    };
    Constructor {
        type_id: "SettlementDecision".into(),
        variant: variant.into(),
        fields: fields.iter().map(|f| (*f).into()).collect(),
        bindings: Default::default(),
    }
}
const VARIANTS: [&str; 5] = ["absent", "valid", "rejected", "conflicting", "not_applicable"];

#[test]
fn c_owns_validity_exact_counterexample_in_v_is_rejected() {
    // Exact counterexample from report C1-R8-01: V-PRESENCE-1 outputs -> SettlementDecision.valid.
    let mut s = spec();
    let rule = s.rules.iter_mut().find(|r| r.id == "V-PRESENCE-1").unwrap();
    rule.outputs = vec![decision("valid")];
    let error = validate(&s).unwrap_err();
    assert!(error.contains("C-validity owner violation"), "{error}");
    assert!(error.contains("stage V") && error.contains("V-PRESENCE-1"), "{error}");
    assert!(error.contains("SettlementDecision"), "{error}");
}

#[test]
fn every_non_c_rule_and_every_variant_is_rejected_while_c_passes() {
    let base = spec();
    validate(&base).unwrap();
    let mut non_c_rules = 0;
    for rule in base.rules.iter().filter(|r| r.stage != "C") {
        for variant in VARIANTS {
            let mut s = base.clone();
            let r = s.rules.iter_mut().find(|x| x.id == rule.id).unwrap();
            r.outputs.push(decision(variant));
            let error = validate(&s).unwrap_err();
            assert!(
                error.contains("C-validity owner violation")
                    && error.contains(&format!("stage {}", rule.stage))
                    && error.contains(&rule.id),
                "{} {variant}: {error}",
                rule.id
            );
        }
        non_c_rules += 1;
    }
    assert!(non_c_rules >= 30, "non-C rules covered: {non_c_rules}");
    for variant in VARIANTS {
        let mut s = base.clone();
        s.rules
            .iter_mut()
            .find(|r| r.id == "C-KEY-1")
            .unwrap()
            .outputs
            .push(decision(variant));
        validate(&s).unwrap_or_else(|e| panic!("valid C construction {variant}: {e}"));
    }
}

#[test]
fn s_forwarding_of_the_c_reference_still_passes_and_is_typed() {
    let s = spec();
    validate(&s).unwrap();
    let forward = s.rules.iter().find(|r| r.id == "S-unique-one-fail").unwrap();
    assert_eq!(forward.outputs[0].type_id, "SettlementForward");
    let mut bad = s;
    bad.rules
        .iter_mut()
        .find(|r| r.id == "S-unique-one-fail")
        .unwrap()
        .outputs[0]
        .bindings
        .insert("validity_ref".into(), json!("S.own_decision"));
    assert!(validate(&bad).unwrap_err().contains("forward immutable validity_ref"));
}

#[test]
fn a_type_carrying_a_c_decision_is_c_owned_too() {
    let mut s = spec();
    s.types.push(TypeDef {
        id: "ValidityEnvelope".into(),
        revision: 1,
        variants: vec![Variant {
            name: "wrap".into(),
            fields: [("inner".to_string(), "SettlementDecision".to_string())].into(),
        }],
    });
    s.projections.push(Projection {
        id: "PROJECT-ValidityEnvelope-wrap".into(),
        revision: 1,
        type_id: "ValidityEnvelope".into(),
        variant: "wrap".into(),
        external_state: "wrap".into(),
        wire_tag: "variant".into(),
        wire_value: json!("wrap"),
        fields: [(
            "inner".to_string(),
            ProjectionField {
                ty: "SettlementDecision".into(),
                source: "inner".into(),
                allowed: None,
            },
        )]
        .into(),
        refs: vec!["SettlementDecision".into()],
    });
    let envelope = Constructor {
        type_id: "ValidityEnvelope".into(),
        variant: "wrap".into(),
        fields: vec!["inner".into()],
        bindings: Default::default(),
    };
    let mut c = s.clone();
    c.rules.iter_mut().find(|r| r.id == "C-KEY-1").unwrap().outputs.push(envelope.clone());
    validate(&c).unwrap_or_else(|e| panic!("C may construct its own envelope: {e}"));
    let mut v = s;
    v.rules.iter_mut().find(|r| r.id == "V-PRESENCE-1").unwrap().outputs.push(envelope);
    let error = validate(&v).unwrap_err();
    assert!(error.contains("C-validity owner violation") && error.contains("ValidityEnvelope"), "{error}");
}

#[test]
fn only_c_may_declare_the_settlement_validity_output() {
    let base = spec();
    for stage in ["E", "S", "V", "F", "X", "K"] {
        let mut s = base.clone();
        s.stages
            .iter_mut()
            .find(|x| x.id == stage)
            .unwrap()
            .outputs
            .push("settlement_validity".into());
        let error = validate(&s).unwrap_err();
        assert!(
            error.contains("C-validity owner violation") && error.contains(stage),
            "{stage}: {error}"
        );
    }
    for boundary in ["C-primary", "C-auxiliary", "S-consume"] {
        let mut s = base.clone();
        s.boundaries
            .iter_mut()
            .find(|x| x.id == boundary)
            .unwrap()
            .outputs
            .push("settlement_validity".into());
        let error = validate(&s).unwrap_err();
        assert!(error.contains("C-validity owner violation"), "{boundary}: {error}");
    }
    // The same declaration on the real owners is not an owner violation (positive control).
    assert!(base.stages.iter().find(|x| x.id == "C").unwrap().outputs.contains(&"settlement_validity".to_string()));
    assert!(base.boundaries.iter().find(|x| x.id == "C-validity").unwrap().outputs.contains(&"settlement_validity".to_string()));
}

/// The exact parser the CLI uses for every supplied document.
fn parse(text: &str) -> Value {
    parse_json(text.as_bytes()).unwrap()
}
fn ck_schema() -> Value {
    parse(include_str!("../../../spec/ck.schema.json"))
}
fn revision_document(literal: &str) -> Value {
    let mut text = serde_json::to_string(&serde_json::to_value(spec()).unwrap()).unwrap();
    let revision = spec().types[0].revision;
    let id = spec().types[0].id.clone();
    let needle = format!("\"id\":\"{id}\",\"revision\":{revision}");
    assert!(text.contains(&needle), "{needle}");
    text = text.replacen(&needle, &format!("\"id\":\"{id}\",\"revision\":{literal}"), 1);
    parse(&text)
}

#[test]
fn revision_domain_is_exactly_one_to_u32_max_in_schema_and_rust() {
    let schema = ck_schema();
    for (literal, accepted) in [
        ("1", true),
        ("1.0", true),
        ("1e0", true),
        ("10e-1", true),
        ("4294967295", true),
        ("4294967295.0", true),
        ("4.294967295e9", true),
        ("4294967296", false),
        ("4294967296.0", false),
        ("0", false),
        ("-1", false),
        ("1.5", false),
        ("4294967295.0000000000000000001", false),
        ("1e400", false),
        ("true", false),
        ("\"1\"", false),
    ] {
        let document = revision_document(literal);
        let verdict = validate_schema(&schema, &document);
        assert_eq!(verdict.is_ok(), accepted, "schema {literal}: {verdict:?}");
        if accepted {
            let typed: Spec = serde_json::from_value(canonicalize_integers(&document))
                .unwrap_or_else(|e| panic!("typed {literal}: {e}"));
            validate(&typed).unwrap_or_else(|e| panic!("validate {literal}: {e}"));
        } else {
            assert!(verdict.unwrap_err().starts_with("Draft2020-12"), "{literal}");
        }
    }
}

fn bundle_text(literal: &str) -> String {
    let text = r#"{"documents":[{"kind":"accounting_input_proof","id":"input","version":"1.7-s1","lineage_id":"L","origins":[{"origin_id":"o","source_origin_id":"s","original_position":@@,"raw_bytes_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","source_revision_ref":"raw:revision","initial_contract_ref":"c"}],"initial_contracts":[{"contract_id":"c","reference_namespace":"unestablished","duplicate_rule":"same_origin_only"}],"grants":[],"identity_bindings":[],"exact_links":[],"continuity":{"predecessor_subject_refs":[],"preserved_initial_contract_refs":[],"preserved_fact_refs":[],"exact_origin_map_ref":"raw:origin-map"}},{"kind":"accounting_result","id":"result","version":"1.7-s1","input_proof_ref":"input","lineage_id":"L","origins":[{"origin_ref":"o","base_id":"b","evidence_atom_id":"o","active_high_unit_id":"u"}],"bases":[{"base_id":"b","origin_refs":["o"],"atomize":false,"atomize_fact_refs":[],"break_copies":false,"break_copies_fact_refs":[]}],"units":[{"unit_id":"u","state":"active","origin_refs":["o"],"exposure_ref":"computed:exposure"}],"primary_validities":[],"financial_candidates":[],"definite_links":[],"coverage_decisions":[],"low_rollups":[],"continuity_result":"unestablished","failure_refs":[]}]}"#;
    text.replace("@@", literal)
}
fn bundle_with_position(literal: &str) -> Value {
    parse(&bundle_text(literal))
}

#[test]
fn origin_position_domain_is_exactly_zero_to_u64_max_without_panic() {
    for (literal, accepted) in [
        ("0", true),
        ("1.0", true),
        ("1e0", true),
        ("18446744073709551615", true),
        ("18446744073709551615.0", true),
        ("1.8446744073709551615e19", true),
        ("18446744073709551616", false),
        ("18446744073709551616.0", false),
        ("18446744073709551615.5", false),
        ("18446744073709551615.00000000000000000000001", false),
        ("-1", false),
        ("1e400", false),
        ("1e999999999999", false),
        ("false", false),
        ("\"1\"", false),
    ] {
        let verdict = validate_accounting_bundle(&bundle_with_position(literal));
        assert_eq!(verdict.is_ok(), accepted, "{literal}: {verdict:?}");
        if !accepted {
            assert!(verdict.unwrap_err().contains("Draft2020-12"), "{literal}");
        }
    }
}

const I128_MAX: &str = "170141183460469231731687303715884105727";
const I128_MAX_PLUS_ONE: &str = "170141183460469231731687303715884105728";

#[test]
fn nano_string_domain_is_i128_max_in_wire_and_evidence_and_constructors() {
    let s = spec();
    let wire = |nano: &str| {
        json!({"exposure":{"pool":"pi_api","low":{"nano":nano,"basis":"usage_valuation"},"high":{"kind":"at_least","nano":"1","basis":"proxy_reservation"}}})
    };
    for (nano, accepted) in [
        ("0", true),
        (I128_MAX, true),
        (I128_MAX_PLUS_ONE, false),
        ("99999999999999999999999999999999999999", true),
        ("999999999999999999999999999999999999999", false),
        ("00", false),
        ("-1", false),
        ("+1", false),
        ("1.0", false),
        ("1\n", false),
    ] {
        let verdict = validate_wire_document(&s, &wire(nano));
        assert_eq!(verdict.is_ok(), accepted, "wire {nano:?}: {verdict:?}");
        let typed = validate_constructor(&s, "Bound", "bound", &json!({"nano": nano, "basis":"usage_valuation","evidence":[]}));
        assert_eq!(typed.is_ok(), accepted && nano != "1\n", "constructor {nano:?}: {typed:?}");
    }
}

#[test]
fn u64_constructor_fields_use_exact_checked_conversion() {
    let s = spec();
    let present = |value: Value| validate_constructor(&s, "FieldCount", "present", &json!({ "value": value }));
    for literal in ["0", "18446744073709551615", "18446744073709551615.0", "1e0"] {
        present(parse(literal)).unwrap_or_else(|e| panic!("{literal}: {e}"));
    }
    for literal in ["18446744073709551616", "18446744073709551616.0", "-1", "1.5", "true", "\"1\"", "1e400"] {
        let error = present(parse(literal)).unwrap_err();
        assert!(error.contains("does not conform to U64"), "{literal}: {error}");
    }
}

fn json_files(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != ".git" && n != "target") {
                json_files(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "json") {
            out.push(path);
        }
    }
}
fn same_modulo_integral_spelling(a: &Value, b: &Value) -> bool {
    match (a, b) {
        // integers are compared exactly; a value `f64` cannot keep (wide integer, non-integer)
        // is an interned stand-in in `parse_json`, so only its class (integral or not) is compared
        (Value::Number(x), Value::Number(y)) => {
            x == y
                || x.as_f64() == y.as_f64()
                || (x.is_f64()
                    && y.is_f64()
                    && x.as_f64().map(|f| f.fract() == 0.0) == y.as_f64().map(|f| f.fract() == 0.0))
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same_modulo_integral_spelling(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| same_modulo_integral_spelling(v, w)))
        }
        _ => a == b,
    }
}

#[test]
fn exact_parser_agrees_with_serde_json_on_every_repository_json_file() {
    let mut files = vec![];
    json_files(&root(), &mut files);
    assert!(files.len() > 300, "files found: {}", files.len());
    for path in files {
        let bytes = std::fs::read(&path).unwrap();
        let standard = serde_json::from_slice::<Value>(&bytes);
        let exact = parse_json(&bytes);
        match (standard, exact) {
            (Ok(a), Ok(b)) => assert!(same_modulo_integral_spelling(&a, &b), "{}", path.display()),
            (Err(_), Err(_)) => {}
            (a, b) => panic!("{}: serde {:?} exact {:?}", path.display(), a.is_ok(), b.is_ok()),
        }
    }
}

#[test]
fn values_from_the_standard_parser_are_read_at_face_value_and_never_panic() {
    // A `Value` built elsewhere with default serde_json holds f64 for decimal literals; the
    // library API must stay controlled for it (frontier literals are exact only via parse_json).
    let doc = |lit: &str| -> Value {
        serde_json::from_str(&bundle_text(lit)).unwrap()
    };
    assert!(validate_accounting_bundle(&doc("18446744073709551615")).is_ok());
    assert!(validate_accounting_bundle(&doc("1.0")).is_ok());
    for lit in ["18446744073709551616", "18446744073709551616.0", "18446744073709551615.0", "1e300", "1.5", "-1"] {
        let error = validate_accounting_bundle(&doc(lit)).unwrap_err();
        assert!(error.contains("Draft2020-12"), "{lit}: {error}");
    }
}

fn assertion_document(first: &str, second: &str) -> Value {
    parse(&format!(
        r#"{{"kind":"accounting_expected_assertion","id":"x","version":"1.7-s1","raw_subject_ref":"r","policy_refs":[],"authored_by":"a","exact_subject_refs":[],"assertions":[
        {{"claim_id":"c","raw_derivation_refs":[],"negative_control_refs":[],"expected_fields_or_relations":{{"n":{first}}}}},
        {{"claim_id":"c","raw_derivation_refs":[],"negative_control_refs":[],"expected_fields_or_relations":{{"n":{second}}}}}]}}"#
    ))
}

#[test]
fn free_form_unique_items_decide_on_exact_values_not_collapsed_stand_ins() {
    // `Assertion.expected_fields_or_relations` is the one free-form location under `uniqueItems`.
    for (first, second) in [
        ("1e41", "1e42"),
        ("18446744073709551616", "18446744073709551617"),
        ("1e-400", "1e-401"),
        ("0.1", "0.1000000000000000055511151231257827"),
        ("1e41", "-1e41"),
        ("0.5", "1e-400"),
    ] {
        let verdict = validate_accounting_document(&assertion_document(first, second));
        assert!(verdict.is_ok(), "{first} vs {second} are distinct: {verdict:?}");
    }
    for (first, second) in [("1e41", "10e40"), ("1.5", "1.50"), ("1e-400", "1e-400"), ("18446744073709551616", "1.8446744073709551616e19")] {
        let document = assertion_document(first, second);
        assert!(validate_accounting_document(&document).is_err(), "{first} vs {second}");
        let full: Value = parse(include_str!("../../../spec/accounting.schema.json"));
        let branch = json!({"$ref":"#/$defs/AccountingExpectedAssertion","$defs":full["$defs"]});
        let error = validate_schema(&branch, &document).unwrap_err();
        assert!(error.contains("non-unique"), "{first} vs {second}: {}", &error[error.len().saturating_sub(120)..]);
    }
}
