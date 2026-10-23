//! Pointer resolution and upstream classes (contract 4.2 steps 2-5; P1-AC1, AC2, AC4, AC5).

mod support;

use portfolio_decision::check_portfolio_bytes;
use serde_json::{Value, json};
use support::*;

/// Every `(code, at)` the check reports for `doc`.
fn run(doc: &Value) -> Vec<(String, String)> {
    let report = check_portfolio_bytes(&bytes(doc)).expect("the record parses");
    codes_of(&report)
}

fn has(found: &[(String, String)], code: &str, at: &str) -> bool {
    found.iter().any(|(c, a)| c == code && a == at)
}

#[track_caller]
fn expect(found: &[(String, String)], code: &str, at: &str) {
    assert!(
        has(found, code, at),
        "expected {code} at {at}, got {found:?}"
    );
}

#[test]
fn the_bound_world_is_valid() {
    let w = world("valid", false);
    assert_eq!(run(&w.doc), Vec::<(String, String)>::new());
}

#[test]
fn check_never_writes_a_bound_decision() {
    let w = world("non-mutation", false);
    let before: Vec<_> = ["a_decision", "b_decision"]
        .iter()
        .map(|k| {
            (
                std::fs::read(w.path(k)).unwrap(),
                std::fs::metadata(w.path(k)).unwrap().modified().unwrap(),
            )
        })
        .collect();
    assert_eq!(run(&w.doc), Vec::<(String, String)>::new());
    let after: Vec<_> = ["a_decision", "b_decision"]
        .iter()
        .map(|k| {
            (
                std::fs::read(w.path(k)).unwrap(),
                std::fs::metadata(w.path(k)).unwrap().modified().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        before, after,
        "bytes and modification time of the bound decisions are unchanged"
    );
    // One v1 and one v2 decision are bound, each beside a decided packet.
    let v = |k: &str| -> u64 {
        let d: Value = serde_json::from_slice(&std::fs::read(w.path(k)).unwrap()).unwrap();
        d["schema_version"].as_u64().unwrap()
    };
    assert_eq!((v("a_decision"), v("b_decision")), (1, 2));
}

/// Edited bytes under every stored-pointer kind are a `digest_mismatch` at that pointer.
#[test]
fn editing_the_bytes_of_each_pointer_kind_is_a_digest_mismatch() {
    let cases: [(&str, &str); 8] = [
        ("handoff", "strategic_basis.handoff"),
        ("a_packet", "proposal_set.0.packet"),
        ("a_decision", "proposal_set.0.decision"),
        ("b_decision", "proposal_set.1.decision"),
        ("comparison", "evaluation_basis.0"),
        ("set_alpha", "evaluation_basis.1"),
        ("assessment", "readiness_basis.0.assessment"),
        ("evaluation", "readiness_basis.0.assessment.evaluation"),
    ];
    for (key, at) in cases {
        let w = world(&format!("edit-{key}"), false);
        w.overwrite(key, "{\"edited\": true}\n");
        expect(&run(&w.doc), "digest_mismatch", at);
    }
}

#[test]
fn editing_the_superseded_record_is_a_digest_mismatch() {
    let w = world("edit-supersedes", false);
    let mut first = w.doc.clone();
    let prior = w.dir.join("rev-1.json");
    let sha = write_json(&prior, &first);
    let mut second = w.doc.clone();
    set(&mut second, "revision", json!(2));
    set(&mut second, "supersedes", file_ref(&prior, &sha));
    assert_eq!(run(&second), Vec::<(String, String)>::new());
    set(&mut first, "revision", json!(7));
    write_json(&prior, &first);
    expect(&run(&second), "digest_mismatch", "supersedes");
}

#[test]
fn supersession_must_name_the_previous_revision_of_the_same_portfolio() {
    let w = world("supersession", false);
    let prior = w.dir.join("rev-1.json");
    let bind = |doc: &Value| {
        let sha = write_json(&prior, doc);
        let mut next = w.doc.clone();
        set(&mut next, "revision", json!(2));
        set(&mut next, "supersedes", file_ref(&prior, &sha));
        next
    };
    assert_eq!(run(&bind(&w.doc.clone())), Vec::<(String, String)>::new());
    let mut wrong_rev = w.doc.clone();
    set(&mut wrong_rev, "revision", json!(5));
    expect(
        &run(&bind(&wrong_rev)),
        "supersession_invalid",
        "supersedes",
    );
    let mut wrong_id = w.doc.clone();
    set(&mut wrong_id, "portfolio_id", json!("pf-other"));
    expect(&run(&bind(&wrong_id)), "supersession_invalid", "supersedes");
    let not_a_portfolio = w.dir.join("rev-1.json");
    let sha = write_json(&not_a_portfolio, &json!({"hello": 1}));
    let mut next = w.doc.clone();
    set(&mut next, "revision", json!(2));
    set(&mut next, "supersedes", file_ref(&not_a_portfolio, &sha));
    expect(&run(&next), "supersession_invalid", "supersedes");
}

#[test]
fn a_comparison_and_an_assessment_are_not_interchangeable() {
    let w = world("class-swap", false);
    // A comparison file in the readiness basis.
    let mut doc = w.doc.clone();
    set(
        &mut doc,
        "readiness_basis.0.assessment",
        w.reference("comparison"),
    );
    expect(
        &run(&doc),
        "upstream_class_mismatch",
        "readiness_basis.0.assessment",
    );
    // An assessment file tagged as a comparison, and as an estimate set.
    for kind in ["comparison", "estimate_set"] {
        let mut doc = w.doc.clone();
        let a = w.reference("assessment");
        set(
            &mut doc,
            "evaluation_basis.0",
            json!({"kind": kind, "path": a["path"], "sha256": a["sha256"]}),
        );
        expect(&run(&doc), "upstream_class_mismatch", "evaluation_basis.0");
    }
}

#[test]
fn an_estimate_set_tagged_comparison_and_the_reverse_are_class_mismatches() {
    let w = world("kind-swap", false);
    let mut doc = w.doc.clone();
    set(&mut doc, "evaluation_basis.0.kind", json!("estimate_set"));
    expect(&run(&doc), "upstream_class_mismatch", "evaluation_basis.0");
    let mut doc = w.doc.clone();
    set(&mut doc, "evaluation_basis.1.kind", json!("comparison"));
    expect(&run(&doc), "upstream_class_mismatch", "evaluation_basis.1");
}

#[test]
fn a_packet_and_a_decision_are_classified_before_their_fields_are_read() {
    let w = world("packet-class", false);
    // A decision where the packet belongs, a packet where the decision belongs.
    let mut doc = w.doc.clone();
    set(&mut doc, "proposal_set.0.packet", w.reference("a_decision"));
    expect(
        &run(&doc),
        "upstream_class_mismatch",
        "proposal_set.0.packet",
    );
    let mut doc = w.doc.clone();
    set(&mut doc, "proposal_set.0.decision", w.reference("a_packet"));
    expect(
        &run(&doc),
        "upstream_class_mismatch",
        "proposal_set.0.decision",
    );
    // Another proposal's packet and decision.
    let mut doc = w.doc.clone();
    set(&mut doc, "proposal_set.0.packet", w.reference("b_packet"));
    expect(&run(&doc), "proposal_id_mismatch", "proposal_set.0.packet");
    let mut doc = w.doc.clone();
    set(
        &mut doc,
        "proposal_set.0.decision",
        w.reference("b_decision"),
    );
    expect(
        &run(&doc),
        "proposal_id_mismatch",
        "proposal_set.0.decision",
    );
}

#[test]
fn a_decided_packet_binds_its_adjacent_decision_and_no_other_packet_does() {
    let w = world("decision-binding", false);
    let mut doc = w.doc.clone();
    set(&mut doc, "proposal_set.0.decision", Value::Null);
    expect(&run(&doc), "decision_binding", "proposal_set.0.decision");
    let mut doc = w.doc.clone();
    set(
        &mut doc,
        "proposal_set.2.decision",
        w.reference("a_decision"),
    );
    expect(&run(&doc), "decision_binding", "proposal_set.2.decision");
    // A decision that is not beside the packet.
    let elsewhere = w.dir.join("elsewhere/decision.json");
    let sha = write_json(&elsewhere, &decision_json("alpha-one", 1));
    let mut doc = w.doc.clone();
    set(
        &mut doc,
        "proposal_set.0.decision",
        file_ref(&elsewhere, &sha),
    );
    expect(&run(&doc), "decision_binding", "proposal_set.0.decision");
}

#[test]
fn an_assessment_must_be_live_and_serve_the_named_decision() {
    let w = world("assessment", false);
    let ev = w.reference("evaluation");
    for purpose in ["fixture", "demonstration"] {
        let mut doc = w.doc.clone();
        let sha = write_json(
            w.path("assessment"),
            &assessment_json(purpose, "decision-1", ev.clone()),
        );
        set(&mut doc, "readiness_basis.0.assessment.sha256", json!(sha));
        expect(
            &run(&doc),
            "assessment_not_live",
            "readiness_basis.0.assessment",
        );
    }
}

#[test]
fn an_assessment_for_another_decision_is_a_mismatch() {
    let w = world("assessment-decision", false);
    let mut doc = w.doc.clone();
    set(
        &mut doc,
        "readiness_basis.0.decision_id",
        json!("decision-2"),
    );
    expect(
        &run(&doc),
        "readiness_decision_mismatch",
        "readiness_basis.0.decision_id",
    );
}

#[test]
fn an_assessment_whose_own_evaluation_is_missing_is_unresolved() {
    let w = world("assessment-eval", false);
    std::fs::remove_file(w.path("evaluation")).unwrap();
    expect(
        &run(&w.doc),
        "pointer_unresolved",
        "readiness_basis.0.assessment.evaluation",
    );
}

#[test]
fn a_ready_assessment_beside_an_excluded_disposition_is_valid() {
    // P1-AC1: readiness informs; it does not select. The assessment's verdict is `ready`.
    let w = world("ready-but-excluded", false);
    let a: Value = serde_json::from_slice(&std::fs::read(w.path("assessment")).unwrap()).unwrap();
    assert_eq!(a["verdict"], "ready");
    assert_eq!(a["purpose"], "live");
    let mut doc = w.doc.clone();
    set(
        &mut doc,
        "dispositions.0",
        json!({"disposition": "exclude_from_current_portfolio", "proposal_id": "alpha-one",
               "rationale": "capacity", "reason": "not this quarter"}),
    );
    set(
        &mut doc,
        "dispositions.1",
        json!({"disposition": "defer", "proposal_id": "beta-two",
        "rationale": "waits", "revisit_when": ["alpha-one returns"]}),
    );
    assert_eq!(run(&doc), Vec::<(String, String)>::new());
}

#[test]
fn a_comparison_or_estimate_set_bound_to_another_packet_revision_is_refused() {
    let w = world("old-beta", true);
    expect(
        &run(&w.doc),
        "basis_revision_mismatch",
        "evaluation_basis.0",
    );
    let mut doc = w.doc.clone();
    let mut basis = doc["evaluation_basis"].as_array().unwrap().clone();
    basis.push(
        json!({"kind": "estimate_set", "path": s(w.path("set_beta")), "sha256": w.sha("set_beta")}),
    );
    set(&mut doc, "evaluation_basis", json!(basis));
    let found = run(&doc);
    expect(&found, "basis_revision_mismatch", "evaluation_basis.2");
    // alpha-one's set still binds the packet the portfolio binds.
    assert!(
        !has(&found, "basis_revision_mismatch", "evaluation_basis.1"),
        "{found:?}"
    );
}

#[test]
fn an_evaluation_file_that_fails_its_owners_check_is_invalid() {
    let w = world("invalid-basis", false);
    let thin = w.dir.join("thin-comparison.json");
    let sha = write_json(
        &thin,
        &json!({"schema": "proposal-evaluation-comparison-v1"}),
    );
    let mut doc = w.doc.clone();
    set(
        &mut doc,
        "evaluation_basis.0",
        json!({"kind": "comparison", "path": s(&thin), "sha256": sha}),
    );
    let found = run(&doc);
    expect(&found, "evaluation_basis_invalid", "evaluation_basis.0");
    assert!(
        !has(&found, "upstream_class_mismatch", "evaluation_basis.0"),
        "{found:?}"
    );
}

/// P1-AC4: one per-measure relation and its pair outcome edited consistently (the finding is
/// internally coherent) no longer re-derives from the contract and estimate sets.
#[test]
fn a_consistently_edited_comparison_does_not_rederive() {
    let w = world("rederive", false);
    let mut finding: Value =
        serde_json::from_slice(&std::fs::read(w.path("comparison")).unwrap()).unwrap();
    set(&mut finding, "pairs.0.measures.0.relation", json!("worse"));
    set(&mut finding, "pairs.0.outcome", json!("b_dominates_a"));
    set(
        &mut finding,
        "frontier",
        json!({"dominated": ["alpha-one"], "non_dominated": ["beta-two"], "undetermined": []}),
    );
    let sha = write_json(w.path("comparison"), &finding);
    let mut doc = w.doc.clone();
    set(&mut doc, "evaluation_basis.0.sha256", json!(sha));
    expect(
        &run(&doc),
        "evaluation_basis_rederive_mismatch",
        "evaluation_basis.0",
    );
}

#[test]
fn a_stored_path_that_cannot_be_read_is_unresolved() {
    let w = world("unreadable", false);
    let mut doc = w.doc.clone();
    let gone = w.dir.join("gone.json");
    set(&mut doc, "strategic_basis.handoff", file_ref(&gone, SHA_A));
    expect(&run(&doc), "pointer_unresolved", "strategic_basis.handoff");
}

#[test]
fn a_directory_a_fifo_and_an_oversized_file_are_refused_without_hanging() {
    let w = world("not-regular", false);
    let mut doc = w.doc.clone();
    set(&mut doc, "strategic_basis.handoff", file_ref(&w.dir, SHA_A));
    expect(&run(&doc), "pointer_unresolved", "strategic_basis.handoff");

    let fifo = w.dir.join("fifo");
    let status = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo");
    assert!(status.success());
    for at in [
        "strategic_basis.handoff",
        "proposal_set.0.packet",
        "proposal_set.0.decision",
        "evaluation_basis.1",
        "readiness_basis.0.assessment",
    ] {
        let mut doc = w.doc.clone();
        set(&mut doc, &format!("{at}.path"), json!(s(&fifo)));
        set(&mut doc, &format!("{at}.sha256"), json!(SHA_A));
        expect(&run(&doc), "pointer_unresolved", at);
    }

    let big = w.dir.join("big.json");
    let file = std::fs::File::create(&big).unwrap();
    file.set_len(32 * 1024 * 1024 + 1).unwrap();
    let mut doc = w.doc.clone();
    set(&mut doc, "strategic_basis.handoff", file_ref(&big, SHA_A));
    expect(&run(&doc), "pointer_unresolved", "strategic_basis.handoff");
}
