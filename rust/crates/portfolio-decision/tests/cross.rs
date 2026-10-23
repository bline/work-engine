//! Cross-proposal rules (contract 4.1 Unlock and Mutual exclusion, errata 5 and 7; P1-AC1, AC6).
//!
//! Every case runs `check_portfolio_bytes` over a record on disk (`world`), so the unlock edges
//! are compared with real packet bytes.

mod support;

use portfolio_decision::check_portfolio_bytes;
use serde_json::{Value, json};
use support::*;

type Found = Vec<(String, String)>;

fn run(doc: &Value) -> Found {
    codes_of(&check_portfolio_bytes(&bytes(doc)).expect("the record parses"))
}

fn pair(code: &str, at: &str) -> (String, String) {
    (code.to_string(), at.to_string())
}

fn unlock(from: &str, to: &str, edges: Value) -> Value {
    json!({"from": from, "to": to, "relationship_basis": edges,
           "judgment": "alpha lands first", "attributed_to": "owner"})
}

fn edge(source: &str, kind: &str, target: &str) -> Value {
    json!({"source": source, "type": kind, "target": target})
}

fn exclusion(members: Value) -> Value {
    json!({"proposals": members, "basis_kind": "capacity", "reason": "one team",
           "attributed_to": "owner"})
}

fn with_unlock(w: &mut World, entries: Value) {
    set(&mut w.doc, "cross_proposal_analysis.unlock", entries);
}

fn with_exclusion(w: &mut World, entries: Value) {
    set(
        &mut w.doc,
        "cross_proposal_analysis.mutually_exclusive_under_basis",
        entries,
    );
}

/// Rewrite the packet of gamma-three with one relationship and rebind it in the record. The
/// comparison and estimate set bind only alpha-one and beta-two, so nothing else goes stale.
fn gamma_edge(w: &mut World, kind: &str, target: &str) {
    let path = w.path("g_packet").to_path_buf();
    let sha = write_json(
        &path,
        &packet_json(
            "gamma-three",
            "evaluated",
            json!([relationship(kind, target)]),
        ),
    );
    set(&mut w.doc, "proposal_set.2.packet", file_ref(&path, &sha));
}

fn select_campaign(w: &mut World, ids: &[&str]) {
    let rows: Vec<Value> = ["alpha-one", "beta-two", "gamma-three"]
        .iter()
        .map(|id| {
            if ids.contains(id) {
                json!({"disposition": "select_for_campaign", "proposal_id": id,
                       "rationale": "r", "campaign_ref": "campaign-1"})
            } else {
                json!({"disposition": "defer", "proposal_id": id,
                       "rationale": "r", "revisit_when": ["later"]})
            }
        })
        .collect();
    set(&mut w.doc, "dispositions", json!(rows));
}

const UNLOCK_0: &str = "cross_proposal_analysis.unlock.0";
const EXCL_0: &str = "cross_proposal_analysis.mutually_exclusive_under_basis.0";

#[test]
fn a_valid_unlock_and_a_valid_exclusion_are_accepted() {
    let mut w = world("cross-valid", false);
    with_unlock(
        &mut w,
        json!([
            unlock(
                "alpha-one",
                "beta-two",
                json!([edge("alpha-one", "enables", "beta-two")])
            ),
            // The endpoints are an unordered pair, and either packet may carry the edge.
            unlock(
                "alpha-one",
                "beta-two",
                json!([edge("beta-two", "depends_on", "alpha-one")])
            ),
            unlock(
                "beta-two",
                "alpha-one",
                json!([edge("alpha-one", "enables", "beta-two")])
            ),
        ]),
    );
    with_exclusion(&mut w, json!([exclusion(json!(["alpha-one", "beta-two"]))]));
    select_campaign(&mut w, &["alpha-one"]);
    assert_eq!(run(&w.doc), Found::new());
}

#[test]
fn a_multi_hop_unlock_is_one_entry_per_hop() {
    let mut w = world("cross-hops", false);
    gamma_edge(&mut w, "informs", "alpha-one");
    with_unlock(
        &mut w,
        json!([
            unlock(
                "gamma-three",
                "alpha-one",
                json!([edge("gamma-three", "informs", "alpha-one")])
            ),
            unlock(
                "alpha-one",
                "beta-two",
                json!([edge("alpha-one", "enables", "beta-two")])
            ),
        ]),
    );
    assert_eq!(run(&w.doc), Found::new());
}

/// One rejecting case: the record, and exactly the findings it must produce.
struct Case {
    label: &'static str,
    doc: Value,
    expect: Found,
}

/// A source-level floor on the cases this file runs (EXT-005): the count comes from this
/// constant, not from anything a fixture states about itself.
const MIN_REJECTING_CASES: usize = 17;

fn rejecting_cases() -> Vec<Case> {
    let mut cases = Vec::new();
    let mut add = |label: &'static str, w: World, expect: Found| {
        cases.push(Case {
            label,
            doc: w.doc,
            expect,
        });
    };

    // Unlock.
    let mut w = world("cross-absent", false);
    with_unlock(
        &mut w,
        json!([unlock(
            "alpha-one",
            "beta-two",
            json!([edge("alpha-one", "depends_on", "beta-two")])
        )]),
    );
    add(
        "edge absent from the packet bytes of its source",
        w,
        vec![pair(
            "relationship_basis_absent",
            &format!("{UNLOCK_0}.relationship_basis.0"),
        )],
    );

    let mut w = world("cross-related-to", false);
    gamma_edge(&mut w, "related_to", "alpha-one");
    with_unlock(
        &mut w,
        json!([unlock(
            "gamma-three",
            "alpha-one",
            json!([edge("gamma-three", "related_to", "alpha-one")])
        )]),
    );
    add(
        "edge of type related_to, present in the packet bytes",
        w,
        vec![pair(
            "unlock_type_invalid",
            &format!("{UNLOCK_0}.relationship_basis.0.type"),
        )],
    );

    let mut w = world("cross-disconnected", false);
    gamma_edge(&mut w, "informs", "alpha-one");
    with_unlock(
        &mut w,
        json!([unlock(
            "alpha-one",
            "beta-two",
            json!([edge("gamma-three", "informs", "alpha-one")])
        )]),
    );
    add(
        "edge exists in the packet bytes but connects other proposals",
        w,
        vec![pair(
            "relationship_basis_disconnected",
            &format!("{UNLOCK_0}.relationship_basis.0"),
        )],
    );

    let mut w = world("cross-from-unknown", false);
    with_unlock(
        &mut w,
        json!([unlock(
            "zeta-nine",
            "beta-two",
            json!([edge("zeta-nine", "enables", "beta-two")])
        )]),
    );
    add(
        "unlock `from` is not in the set",
        w,
        vec![
            pair("unlock_endpoint_unknown", &format!("{UNLOCK_0}.from")),
            pair(
                "relationship_basis_absent",
                &format!("{UNLOCK_0}.relationship_basis.0"),
            ),
        ],
    );

    let mut w = world("cross-to-unknown", false);
    with_unlock(
        &mut w,
        json!([unlock(
            "alpha-one",
            "zeta-nine",
            json!([edge("alpha-one", "enables", "beta-two")])
        )]),
    );
    add(
        "unlock `to` is not in the set",
        w,
        vec![
            pair("unlock_endpoint_unknown", &format!("{UNLOCK_0}.to")),
            pair(
                "relationship_basis_disconnected",
                &format!("{UNLOCK_0}.relationship_basis.0"),
            ),
        ],
    );

    let mut w = world("cross-empty-basis", false);
    with_unlock(&mut w, json!([unlock("alpha-one", "beta-two", json!([]))]));
    add(
        "an unlock with no relationship_basis edge",
        w,
        vec![pair("schema", &format!("{UNLOCK_0}.relationship_basis"))],
    );

    let mut w = world("cross-source-unknown", false);
    with_unlock(
        &mut w,
        json!([unlock(
            "alpha-one",
            "beta-two",
            json!([edge("zeta-nine", "enables", "beta-two")])
        )]),
    );
    add(
        "edge source is not in the set",
        w,
        vec![
            pair(
                "relationship_basis_absent",
                &format!("{UNLOCK_0}.relationship_basis.0"),
            ),
            pair(
                "relationship_basis_disconnected",
                &format!("{UNLOCK_0}.relationship_basis.0"),
            ),
        ],
    );

    let mut w = world("cross-second-edge", false);
    with_unlock(
        &mut w,
        json!([unlock(
            "alpha-one",
            "beta-two",
            json!([
                edge("alpha-one", "enables", "beta-two"),
                edge("beta-two", "informs", "alpha-one")
            ])
        )]),
    );
    add(
        "the second edge of an unlock is checked on its own",
        w,
        vec![pair(
            "relationship_basis_absent",
            &format!("{UNLOCK_0}.relationship_basis.1"),
        )],
    );

    let mut w = world("cross-empty-judgment", false);
    with_unlock(
        &mut w,
        json!([unlock(
            "alpha-one",
            "beta-two",
            json!([edge("alpha-one", "enables", "beta-two")])
        )]),
    );
    set(
        &mut w.doc,
        "cross_proposal_analysis.unlock.0.judgment",
        json!(""),
    );
    set(
        &mut w.doc,
        "cross_proposal_analysis.unlock.0.attributed_to",
        json!(""),
    );
    add(
        "empty judgment and attribution on an unlock",
        w,
        vec![
            pair("empty_string", &format!("{UNLOCK_0}.judgment")),
            pair("empty_string", &format!("{UNLOCK_0}.attributed_to")),
        ],
    );

    // Mutual exclusion.
    let mut w = world("cross-one-member", false);
    with_exclusion(&mut w, json!([exclusion(json!(["alpha-one"]))]));
    add(
        "an exclusion with one member",
        w,
        vec![pair(
            "exclusion_member_invalid",
            &format!("{EXCL_0}.proposals"),
        )],
    );

    let mut w = world("cross-member-unknown", false);
    with_exclusion(
        &mut w,
        json!([exclusion(json!(["alpha-one", "zeta-nine"]))]),
    );
    add(
        "an exclusion member outside the set",
        w,
        vec![pair(
            "exclusion_member_invalid",
            &format!("{EXCL_0}.proposals.1"),
        )],
    );

    let mut w = world("cross-member-duplicate", false);
    with_exclusion(
        &mut w,
        json!([exclusion(json!(["alpha-one", "alpha-one"]))]),
    );
    add(
        "an exclusion member listed twice",
        w,
        vec![pair(
            "exclusion_member_invalid",
            &format!("{EXCL_0}.proposals.1"),
        )],
    );

    let mut w = world("cross-contradicted", false);
    with_exclusion(&mut w, json!([exclusion(json!(["alpha-one", "beta-two"]))]));
    select_campaign(&mut w, &["alpha-one", "beta-two"]);
    add(
        "two members of one exclusion both select_for_campaign",
        w,
        vec![pair("exclusion_contradicted", EXCL_0)],
    );

    let mut w = world("cross-contradicted-three", false);
    with_exclusion(
        &mut w,
        json!([exclusion(json!(["alpha-one", "beta-two", "gamma-three"]))]),
    );
    select_campaign(&mut w, &["alpha-one", "gamma-three"]);
    add(
        "two of three members select_for_campaign",
        w,
        vec![pair("exclusion_contradicted", EXCL_0)],
    );

    let mut w = world("cross-second-entry", false);
    with_exclusion(
        &mut w,
        json!([
            exclusion(json!(["alpha-one", "gamma-three"])),
            exclusion(json!(["alpha-one", "beta-two"]))
        ]),
    );
    select_campaign(&mut w, &["alpha-one", "beta-two"]);
    add(
        "the contradicted entry is the second one",
        w,
        vec![pair(
            "exclusion_contradicted",
            "cross_proposal_analysis.mutually_exclusive_under_basis.1",
        )],
    );

    let mut w = world("cross-empty-reason", false);
    with_exclusion(&mut w, json!([exclusion(json!(["alpha-one", "beta-two"]))]));
    set(
        &mut w.doc,
        "cross_proposal_analysis.mutually_exclusive_under_basis.0.reason",
        json!(""),
    );
    set(
        &mut w.doc,
        "cross_proposal_analysis.mutually_exclusive_under_basis.0.attributed_to",
        json!(""),
    );
    add(
        "empty reason and attribution on an exclusion",
        w,
        vec![
            pair("empty_string", &format!("{EXCL_0}.reason")),
            pair("empty_string", &format!("{EXCL_0}.attributed_to")),
        ],
    );

    // Counter-case: an excluded member beside a selected one is not a contradiction.
    let mut w = world("cross-exclude-not-select", false);
    with_exclusion(&mut w, json!([exclusion(json!(["alpha-one", "beta-two"]))]));
    set(
        &mut w.doc,
        "dispositions.0",
        json!({"disposition": "exclude_from_current_portfolio", "proposal_id": "alpha-one",
               "rationale": "r", "reason": "capacity"}),
    );
    set(
        &mut w.doc,
        "dispositions.1",
        json!({"disposition": "select_for_campaign",
        "proposal_id": "beta-two", "rationale": "r", "campaign_ref": "c"}),
    );
    add(
        "exclusion with one selected and one excluded member is valid",
        w,
        Vec::new(),
    );

    cases
}

#[test]
fn every_cross_proposal_case_reports_exactly_its_findings() {
    let cases = rejecting_cases();
    assert!(
        cases.len() >= MIN_REJECTING_CASES,
        "{} cases, expected at least {MIN_REJECTING_CASES}",
        cases.len()
    );
    let mut ran = 0;
    for case in &cases {
        assert_eq!(run(&case.doc), case.expect, "{}", case.label);
        ran += 1;
    }
    assert_eq!(ran, cases.len());
}

#[test]
fn a_duplicated_member_is_not_two_selected_members() {
    let mut w = world("cross-duplicate-one-select", false);
    with_exclusion(
        &mut w,
        json!([exclusion(json!(["alpha-one", "alpha-one"]))]),
    );
    select_campaign(&mut w, &["alpha-one"]);
    assert_eq!(
        run(&w.doc),
        vec![pair(
            "exclusion_member_invalid",
            &format!("{EXCL_0}.proposals.1")
        )]
    );
}

#[test]
fn an_unread_packet_is_not_reported_as_missing_an_edge() {
    // The packet of alpha-one is edited after it was bound: the pointer rules report the
    // digest, and the unlock does not also claim that the edge is absent (not established).
    let mut w = world("cross-unread", false);
    with_unlock(
        &mut w,
        json!([unlock(
            "alpha-one",
            "beta-two",
            json!([edge("alpha-one", "depends_on", "beta-two")])
        )]),
    );
    w.overwrite("a_packet", "{}");
    let found = run(&w.doc);
    assert!(
        found.iter().any(|(c, _)| c == "digest_mismatch"),
        "{found:?}"
    );
    assert!(
        !found.iter().any(|(c, _)| c == "relationship_basis_absent"),
        "{found:?}"
    );
}

/// The codes of contract 4.4 plus errata 3-7. P1-AC6 asks for a rejecting test per code; the
/// check below finds each code's literal in the test sources of this crate. A literal is a
/// pointer to a test, not proof of one (the cases above and in `structure.rs` and `upstream.rs`
/// assert `(code, at)` pairs), so this list also pins the number of codes.
const CONTRACT_CODES: [&str; 21] = [
    "supersession_invalid",
    "readiness_proposal_unknown",
    "disposition_missing",
    "disposition_duplicate",
    "disposition_unknown_proposal",
    "rank_duplicate",
    "sequence_target_invalid",
    "sequence_cycle",
    "unlock_endpoint_unknown",
    "relationship_basis_absent",
    "unlock_type_invalid",
    "exclusion_member_invalid",
    "basis_revision_mismatch",
    "evaluation_basis_rederive_mismatch",
    "evaluation_basis_invalid",
    "assessment_not_live",
    "proposal_id_mismatch",
    "decision_binding",
    "relationship_basis_disconnected",
    "readiness_decision_mismatch",
    "exclusion_contradicted",
];

#[test]
fn every_contract_code_is_declared_and_has_a_rejecting_test_source() {
    use portfolio_decision::codes as c;
    let declared = [
        c::SUPERSESSION_INVALID,
        c::READINESS_PROPOSAL_UNKNOWN,
        c::DISPOSITION_MISSING,
        c::DISPOSITION_DUPLICATE,
        c::DISPOSITION_UNKNOWN_PROPOSAL,
        c::RANK_DUPLICATE,
        c::SEQUENCE_TARGET_INVALID,
        c::SEQUENCE_CYCLE,
        c::UNLOCK_ENDPOINT_UNKNOWN,
        c::RELATIONSHIP_BASIS_ABSENT,
        c::UNLOCK_TYPE_INVALID,
        c::EXCLUSION_MEMBER_INVALID,
        c::BASIS_REVISION_MISMATCH,
        c::EVALUATION_BASIS_REDERIVE_MISMATCH,
        c::EVALUATION_BASIS_INVALID,
        c::ASSESSMENT_NOT_LIVE,
        c::PROPOSAL_ID_MISMATCH,
        c::DECISION_BINDING,
        c::RELATIONSHIP_BASIS_DISCONNECTED,
        c::READINESS_DECISION_MISMATCH,
        c::EXCLUSION_CONTRADICTED,
    ];
    assert_eq!(
        declared, CONTRACT_CODES,
        "declared constants, in list order"
    );
    let sources = [
        include_str!("structure.rs"),
        include_str!("upstream.rs"),
        include_str!("cross.rs"),
    ];
    assert_eq!(CONTRACT_CODES.len(), 21);
    for code in CONTRACT_CODES {
        let quoted = format!("\"{code}\"");
        let uses: usize = sources
            .iter()
            .map(|s| s.matches(quoted.as_str()).count())
            .sum();
        // The list above is one use in this file; a rejecting assertion is another.
        assert!(
            uses >= 2,
            "{code} has no rejecting assertion ({uses} literals)"
        );
    }
}
