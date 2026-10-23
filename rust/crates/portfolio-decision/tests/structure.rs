//! Unit 2: model and structural rules (S4, S10, P1-AC3, part of P1-AC6).

mod support;

use portfolio_decision::model::{DISPOSITION_TAGS, Disposition, PortfolioDecision};
use portfolio_decision::structure::{parse_portfolio, validate};
use serde_json::{Value, json};
use support::*;

/// Structural findings of `doc` as `(code, at)` pairs.
fn findings(doc: &Value) -> Vec<(String, String)> {
    match parse_portfolio(&bytes(doc)).expect("json") {
        Err(errors) => errors.into_iter().map(|e| (e.code, e.at)).collect(),
        Ok(p) => validate(&p).into_iter().map(|e| (e.code, e.at)).collect(),
    }
}

fn one(doc: &Value, code: &str, at: &str) {
    assert_eq!(
        findings(doc),
        vec![(code.to_string(), at.to_string())],
        "{doc:#}"
    );
}

#[test]
fn the_sample_record_is_structurally_valid_and_round_trips() {
    let doc = sample();
    assert_eq!(findings(&doc), vec![]);
    let parsed: PortfolioDecision = serde_json::from_slice(&bytes(&doc)).unwrap();
    let again: Value = serde_json::to_value(&parsed).unwrap();
    assert_eq!(
        again, doc,
        "serialisation preserves every field and the declared order"
    );
}

#[test]
fn schema_const_is_checked_before_anything_else() {
    let mut doc = sample();
    set(&mut doc, "schema", json!("portfolio-decision-v2"));
    one(&doc, "schema_mismatch", "schema");
    set(&mut doc, "schema", json!(7));
    one(&doc, "schema", "schema");
}

#[test]
fn duplicate_keys_and_floats_are_schema_errors() {
    let text = serde_json::to_string(&sample()).unwrap();
    let dup = text.replacen("\"revision\":1", "\"revision\":1,\"revision\":1", 1);
    let report = parse_portfolio(dup.as_bytes()).unwrap().unwrap_err();
    assert_eq!(report[0].code, "schema");
    let mut doc = sample();
    set(&mut doc, "revision", json!(1.5));
    assert_eq!(findings(&doc)[0].0, "schema", "no float type can hold 1.5");
    set(&mut doc, "revision", json!(1e30));
    assert_eq!(findings(&doc)[0].0, "schema");
}

#[test]
fn not_json_is_a_failure_not_a_report() {
    assert!(parse_portfolio(b"{not json").is_err());
}

#[test]
fn unknown_keys_are_refused_in_every_record_nested_record_and_variant() {
    let cases = [
        "",
        "authority",
        "strategic_basis",
        "strategic_basis.handoff",
        "supersedes_x",
        "proposal_set.0",
        "proposal_set.0.packet",
        "relationship_graph",
        "cross_proposal_analysis",
        "dispositions.0",
        "dispositions.1",
        "dispositions.2",
        "non_effects",
    ];
    for path in cases {
        let mut doc = sample();
        let target = if path.is_empty() || path == "supersedes_x" {
            &mut doc
        } else {
            pointer_mut(&mut doc, path)
        };
        target
            .as_object_mut()
            .unwrap()
            .insert("surprise".into(), json!(1));
        let found = findings(&doc);
        assert_eq!(found.len(), 1, "{path}: {found:?}");
        assert_eq!(found[0].0, "schema", "{path}");
    }
}

#[test]
fn unknown_keys_are_refused_inside_the_populated_bases_and_cross_proposal_records() {
    let mut doc = sample();
    set(
        &mut doc,
        "evaluation_basis",
        json!([{"kind": "comparison", "path": "/x/c.json", "sha256": SHA_A, "extra": 1}]),
    );
    assert_eq!(findings(&doc)[0].0, "schema");
    let mut doc = sample();
    set(
        &mut doc,
        "readiness_basis",
        json!([{"proposal_id": "alpha-one", "decision_id": "d", "assessment": {"path": "/x/a.json", "sha256": SHA_A, "extra": 1}}]),
    );
    assert_eq!(findings(&doc)[0].0, "schema");
    let mut doc = sample();
    set(
        &mut doc,
        "cross_proposal_analysis.unlock",
        json!([{"from": "alpha-one", "to": "beta-two", "relationship_basis": [{"source": "alpha-one", "type": "enables", "target": "beta-two", "extra": 1}], "judgment": "j", "attributed_to": "a"}]),
    );
    assert_eq!(findings(&doc)[0].0, "schema");
    let mut doc = sample();
    set(
        &mut doc,
        "cross_proposal_analysis.mutually_exclusive_under_basis",
        json!([{"proposals": ["alpha-one", "beta-two"], "basis_kind": "capacity", "reason": "r", "attributed_to": "a", "extra": 1}]),
    );
    assert_eq!(findings(&doc)[0].0, "schema");
    let mut doc = sample();
    set(
        &mut doc,
        "cross_proposal_analysis.mutually_exclusive_under_basis",
        json!([{"proposals": ["alpha-one", "beta-two"], "basis_kind": "conflicts_with", "reason": "r", "attributed_to": "a"}]),
    );
    assert_eq!(findings(&doc)[0].0, "schema", "basis_kind is a closed enum");
}

#[test]
fn nullable_fields_are_required_keys() {
    let mut doc = sample();
    remove(&mut doc, "supersedes");
    assert_eq!(findings(&doc)[0].0, "schema");
    let mut doc = sample();
    remove(&mut doc, "proposal_set.0.decision");
    assert_eq!(findings(&doc)[0].0, "schema");
}

#[test]
fn unknown_disposition_and_relationship_source_are_closed_enums() {
    let mut doc = sample();
    set(&mut doc, "dispositions.0.disposition", json!("reject"));
    assert_eq!(findings(&doc)[0].0, "schema");
    let mut doc = sample();
    set(
        &mut doc,
        "dispositions.0.disposition",
        json!("not_selected"),
    );
    assert_eq!(
        findings(&doc)[0].0,
        "schema",
        "not_selected is not a variant (contract 4.1 note)"
    );
    let mut doc = sample();
    set(
        &mut doc,
        "relationship_graph.source",
        json!("somewhere_else"),
    );
    assert_eq!(findings(&doc)[0].0, "schema");
}

#[test]
fn identity_rules() {
    let mut doc = sample();
    set(&mut doc, "portfolio_id", json!("Bad Id"));
    one(&doc, "id_pattern", "portfolio_id");
    let mut doc = sample();
    set(&mut doc, "decided_at", json!("2026-10-23"));
    one(&doc, "time_format", "decided_at");
    let mut doc = sample();
    set(&mut doc, "revision", json!(0));
    let found = findings(&doc);
    assert!(
        found.contains(&("schema".into(), "revision".into())),
        "{found:?}"
    );
    let mut doc = sample();
    set(&mut doc, "authority.evidence", json!(""));
    one(&doc, "empty_string", "authority.evidence");
    let mut doc = sample();
    set(
        &mut doc,
        "relationship_graph.repository_revision",
        json!("ABC"),
    );
    one(&doc, "schema", "relationship_graph.repository_revision");
}

#[test]
fn supersedes_is_null_iff_revision_is_one() {
    let mut doc = sample();
    set(
        &mut doc,
        "supersedes",
        json!({"path": "/x/prev.json", "sha256": SHA_B}),
    );
    one(&doc, "supersession_invalid", "supersedes");
    let mut doc = sample();
    set(&mut doc, "revision", json!(2));
    one(&doc, "supersession_invalid", "supersedes");
    set(
        &mut doc,
        "supersedes",
        json!({"path": "/x/prev.json", "sha256": SHA_B}),
    );
    assert_eq!(findings(&doc), vec![]);
}

#[test]
fn proposal_set_rules() {
    let mut doc = sample();
    set(&mut doc, "proposal_set.1.proposal_id", json!("alpha-one"));
    let found = findings(&doc);
    assert!(found.contains(&(
        "duplicate_proposal_id".into(),
        "proposal_set.1.proposal_id".into()
    )));
    let mut doc = sample();
    set(&mut doc, "proposal_set.0.proposal_id", json!("nosep"));
    assert!(findings(&doc).contains(&("id_pattern".into(), "proposal_set.0.proposal_id".into())));
    let mut doc = sample();
    set(&mut doc, "proposal_set", json!([]));
    assert!(findings(&doc).contains(&("schema".into(), "proposal_set".into())));
    let mut doc = sample();
    set(
        &mut doc,
        "proposal_set.0.packet.path",
        json!("relative/packet.json"),
    );
    one(&doc, "path_not_normalised", "proposal_set.0.packet.path");
    let mut doc = sample();
    set(&mut doc, "proposal_set.0.packet.sha256", json!("XYZ"));
    one(&doc, "sha256_pattern", "proposal_set.0.packet.sha256");
}

#[test]
fn readiness_basis_proposal_must_be_in_the_set() {
    let mut doc = sample();
    set(
        &mut doc,
        "readiness_basis",
        json!([{"proposal_id": "zeta-nine", "decision_id": "d1", "assessment": {"path": "/x/a.json", "sha256": SHA_A}}]),
    );
    let mut limits = doc["limitations"].as_array().unwrap().clone();
    limits.push(json!("readiness_decision_fit_unverified: author judgment"));
    set(&mut doc, "limitations", Value::Array(limits));
    one(
        &doc,
        "readiness_proposal_unknown",
        "readiness_basis.0.proposal_id",
    );
}

#[test]
fn every_proposal_has_exactly_one_disposition() {
    let mut doc = sample();
    let two = json!([
        doc["dispositions"][0].clone(),
        doc["dispositions"][1].clone()
    ]);
    set(&mut doc, "dispositions", two);
    one(&doc, "disposition_missing", "proposal_set.2");
    let mut doc = sample();
    set(&mut doc, "dispositions.2.proposal_id", json!("alpha-one"));
    let found = findings(&doc);
    assert!(
        found.contains(&(
            "disposition_duplicate".into(),
            "dispositions.2.proposal_id".into()
        )),
        "{found:?}"
    );
    assert!(found.contains(&("disposition_missing".into(), "proposal_set.2".into())));
    let mut doc = sample();
    set(&mut doc, "dispositions.2.proposal_id", json!("zeta-nine"));
    let found = findings(&doc);
    assert!(found.contains(&(
        "disposition_unknown_proposal".into(),
        "dispositions.2.proposal_id".into()
    )));
}

#[test]
fn ranks_are_unique_and_at_least_one() {
    let mut doc = sample();
    set(
        &mut doc,
        "dispositions.2",
        json!({"disposition": "prioritize", "proposal_id": "gamma-three", "rationale": "r", "rank": 1}),
    );
    one(&doc, "rank_duplicate", "dispositions.2.rank");
    set(&mut doc, "dispositions.2.rank", json!(0));
    one(&doc, "schema", "dispositions.2.rank");
    set(&mut doc, "dispositions.2.rank", json!(-1));
    assert_eq!(findings(&doc)[0].0, "schema", "rank is u32");
    set(&mut doc, "dispositions.2.rank", json!(2));
    assert_eq!(findings(&doc), vec![]);
}

#[test]
fn sequence_targets_and_cycles() {
    let mut doc = sample();
    set(&mut doc, "dispositions.1.after", json!(["beta-two"]));
    one(&doc, "sequence_target_invalid", "dispositions.1.after.0");
    set(&mut doc, "dispositions.1.after", json!(["zeta-nine"]));
    one(&doc, "sequence_target_invalid", "dispositions.1.after.0");
    set(&mut doc, "dispositions.1.after", json!([]));
    one(&doc, "sequence_target_invalid", "dispositions.1.after");
    // alpha -> beta -> alpha
    let mut doc = sample();
    set(
        &mut doc,
        "dispositions.0",
        json!({"disposition": "sequence", "proposal_id": "alpha-one", "rationale": "r", "after": ["beta-two"]}),
    );
    set(&mut doc, "dispositions.1.after", json!(["alpha-one"]));
    let found = findings(&doc);
    assert_eq!(
        found,
        vec![
            ("sequence_cycle".to_string(), "dispositions.0".to_string()),
            ("sequence_cycle".to_string(), "dispositions.1".to_string())
        ]
    );
    // a longer cycle leaves the node outside it alone
    let mut doc = sample();
    set(
        &mut doc,
        "dispositions",
        json!([
            {"disposition": "sequence", "proposal_id": "alpha-one", "rationale": "r", "after": ["beta-two"]},
            {"disposition": "sequence", "proposal_id": "beta-two", "rationale": "r", "after": ["gamma-three"]},
            {"disposition": "sequence", "proposal_id": "gamma-three", "rationale": "r", "after": ["beta-two"]}
        ]),
    );
    let found = findings(&doc);
    assert_eq!(
        found,
        vec![
            ("sequence_cycle".to_string(), "dispositions.1".to_string()),
            ("sequence_cycle".to_string(), "dispositions.2".to_string())
        ]
    );
}

#[test]
fn defer_exclude_and_select_need_their_fields() {
    let mut doc = sample();
    set(&mut doc, "dispositions.2.revisit_when", json!([]));
    one(&doc, "schema", "dispositions.2.revisit_when");
    set(&mut doc, "dispositions.2.revisit_when", json!([""]));
    one(&doc, "empty_string", "dispositions.2.revisit_when.0");
    let mut doc = sample();
    set(
        &mut doc,
        "dispositions.2",
        json!({"disposition": "exclude_from_current_portfolio", "proposal_id": "gamma-three", "rationale": "r", "reason": ""}),
    );
    one(&doc, "empty_string", "dispositions.2.reason");
    set(
        &mut doc,
        "dispositions.2",
        json!({"disposition": "select_for_campaign", "proposal_id": "gamma-three", "rationale": "r", "campaign_ref": ""}),
    );
    one(&doc, "empty_string", "dispositions.2.campaign_ref");
    set(&mut doc, "dispositions.2.rationale", json!(""));
    assert!(findings(&doc).contains(&("empty_string".into(), "dispositions.2.rationale".into())));
    let mut doc = sample();
    remove(&mut doc, "dispositions.0.rank");
    assert_eq!(
        findings(&doc)[0].0,
        "schema",
        "a variant's own field is required"
    );
}

#[test]
fn non_effects_must_all_be_false() {
    for name in [
        "proposal_lifecycle_mutated",
        "proposal_rejected",
        "relationship_graph_mutated",
        "implementation_authorized",
        "schedule_executed",
    ] {
        let mut doc = sample();
        set(&mut doc, &format!("non_effects.{name}"), json!(true));
        one(&doc, "authority_asserted", &format!("non_effects.{name}"));
    }
}

#[test]
fn required_limitations() {
    for (drop, expected) in [
        ("strategic_basis_unverified", 1),
        ("relationship_graph_revision_unverified", 1),
        ("no_evaluation_basis", 1),
    ] {
        let mut doc = sample();
        let kept: Vec<Value> = doc["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|l| !l.as_str().unwrap().starts_with(drop))
            .cloned()
            .collect();
        set(&mut doc, "limitations", Value::Array(kept));
        let found = findings(&doc);
        assert_eq!(
            found,
            vec![("limitation_missing".to_string(), "limitations".to_string())],
            "{drop}"
        );
        assert_eq!(found.len(), expected);
    }
    // no_evaluation_basis is only required while evaluation_basis is empty
    let mut doc = sample();
    set(
        &mut doc,
        "evaluation_basis",
        json!([{"kind": "estimate_set", "path": "/x/e.json", "sha256": SHA_A}]),
    );
    one_missing_free(&doc);
    // readiness_decision_fit_unverified is required while readiness_basis is non-empty
    let mut doc = sample();
    set(
        &mut doc,
        "readiness_basis",
        json!([{"proposal_id": "alpha-one", "decision_id": "d1", "assessment": {"path": "/x/a.json", "sha256": SHA_A}}]),
    );
    one(&doc, "limitation_missing", "limitations");
    let mut limits = doc["limitations"].as_array().unwrap().clone();
    limits.push(json!("readiness_decision_fit_unverified: author judgment"));
    set(&mut doc, "limitations", Value::Array(limits));
    assert_eq!(findings(&doc), vec![]);
}

#[test]
fn a_required_limitation_is_matched_by_its_name_not_by_a_prefix() {
    let required = [
        "strategic_basis_unverified",
        "relationship_graph_revision_unverified",
    ];
    for name in required {
        for lookalike in [
            format!("{name}_is_false_here"),
            format!("{name}_is_false_here: the text after is ignored"),
            format!("{name}2: a longer name"),
            format!("x{name}: a shorter-prefixed name"),
            format!("{name} : space before the colon"),
        ] {
            let mut doc = sample();
            let limits: Vec<Value> = doc["limitations"]
                .as_array()
                .unwrap()
                .iter()
                .map(|l| {
                    if l.as_str().unwrap().starts_with(name) {
                        json!(lookalike)
                    } else {
                        l.clone()
                    }
                })
                .collect();
            set(&mut doc, "limitations", Value::Array(limits));
            assert_eq!(
                findings(&doc),
                vec![("limitation_missing".to_string(), "limitations".to_string())],
                "{lookalike:?} must not satisfy {name}"
            );
        }
        // The bare name and the `name: text` form are both accepted.
        let mut doc = sample();
        let limits: Vec<Value> = doc["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| {
                if l.as_str().unwrap().starts_with(name) {
                    json!(name)
                } else {
                    l.clone()
                }
            })
            .collect();
        set(&mut doc, "limitations", Value::Array(limits));
        assert_eq!(findings(&doc), vec![], "the bare name {name}");
    }
}

fn one_missing_free(doc: &Value) {
    assert_eq!(
        findings(doc),
        vec![],
        "an empty-basis limitation does no harm when a basis exists"
    );
}

#[test]
fn disposition_vocabulary_is_disjoint_from_decision_and_lifecycle_vocabularies() {
    let proposal_decision = [
        "defer_for_dogfooding",
        "approve_proposal_meaning",
        "revise_placement",
    ];
    let lifecycle = [
        "formed",
        "placement_uncertain",
        "evaluated",
        "decided",
        "superseded",
    ];
    let serialised: Vec<String> = [
        Disposition::Prioritize {
            proposal_id: "a-b".into(),
            rationale: "r".into(),
            rank: 1,
        },
        Disposition::Sequence {
            proposal_id: "a-b".into(),
            rationale: "r".into(),
            after: vec![],
        },
        Disposition::Defer {
            proposal_id: "a-b".into(),
            rationale: "r".into(),
            revisit_when: vec![],
        },
        Disposition::ExcludeFromCurrentPortfolio {
            proposal_id: "a-b".into(),
            rationale: "r".into(),
            reason: "r".into(),
        },
        Disposition::SelectForCampaign {
            proposal_id: "a-b".into(),
            rationale: "r".into(),
            campaign_ref: "c".into(),
        },
    ]
    .iter()
    .map(|d| {
        let v = serde_json::to_value(d).unwrap();
        assert_eq!(v["disposition"], d.tag());
        d.tag().to_string()
    })
    .collect();
    assert_eq!(
        serialised, DISPOSITION_TAGS,
        "DISPOSITION_TAGS follows the enum"
    );
    for tag in DISPOSITION_TAGS {
        assert!(
            !proposal_decision.contains(&tag),
            "{tag} collides with a proposal-decision disposition"
        );
        assert!(
            !lifecycle.contains(&tag),
            "{tag} collides with a lifecycle state"
        );
    }
}

fn collect_names(value: &Value, path: &str, out: &mut Vec<(String, String)>) {
    match value {
        Value::Object(map) => {
            for (key, v) in map {
                let here = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                out.push((here.clone(), key.clone()));
                collect_names(v, &here, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|v| collect_names(v, path, out)),
        Value::String(s)
            if path.ends_with("disposition")
                || path.ends_with("basis_kind")
                || path.ends_with("kind")
                || path.ends_with("source") =>
        {
            out.push((path.to_string(), s.clone()));
        }
        _ => {}
    }
}

#[test]
fn nothing_in_the_record_vocabulary_can_say_reject() {
    // A record using every variant and every enum value.
    let mut doc = sample();
    set(
        &mut doc,
        "dispositions",
        json!([
            {"disposition": "prioritize", "proposal_id": "alpha-one", "rationale": "r", "rank": 1},
            {"disposition": "sequence", "proposal_id": "beta-two", "rationale": "r", "after": ["alpha-one"]},
            {"disposition": "select_for_campaign", "proposal_id": "gamma-three", "rationale": "r", "campaign_ref": "c"},
        ]),
    );
    set(
        &mut doc,
        "evaluation_basis",
        json!([{"kind": "estimate_set", "path": "/x/e.json", "sha256": SHA_A}]),
    );
    set(
        &mut doc,
        "cross_proposal_analysis.mutually_exclusive_under_basis",
        json!([{"proposals": ["alpha-one", "beta-two"], "basis_kind": "timing", "reason": "r", "attributed_to": "a"}]),
    );
    let parsed: PortfolioDecision = serde_json::from_slice(&bytes(&doc)).unwrap();
    let mut names = Vec::new();
    collect_names(&serde_json::to_value(&parsed).unwrap(), "", &mut names);
    let mut hits: Vec<_> = names.iter().filter(|(_, n)| n.contains("reject")).collect();
    hits.dedup();
    assert_eq!(
        hits.iter().map(|(p, _)| p.as_str()).collect::<Vec<_>>(),
        vec!["non_effects.proposal_rejected"],
        "only the constant-false non-effect may contain 'reject'"
    );
    for tag in DISPOSITION_TAGS {
        assert!(!tag.contains("reject"));
    }
    // And the code vocabulary: no error code names rejection either.
    let source = include_str!("../src/diag.rs");
    assert!(
        !source.to_lowercase().contains("reject"),
        "no code or limitation name may mention rejection"
    );
}
