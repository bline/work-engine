//! The three boundary rules of the comparison (E2 review P3-4), each pinned on both sides of
//! its boundary through the CLI: a confidence equal to the contract minimum is estimated; an
//! evidence cutoff equal to `stale_before` is not stale; intervals that touch at an endpoint
//! overlap, and intervals disjoint by one dominate.

mod support;
use serde_json::{Value, json};
use std::path::Path;
use support::*;

fn edit_json(path: &Path, edit: impl Fn(&mut Value)) {
    let mut doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    edit(&mut doc);
    write_json(path, &doc);
}

/// The finding `compare` prints for the world's two sets.
fn compare(w: &World) -> Value {
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
    assert_eq!(run.code, 0, "{}", run.stderr);
    run.out_json()
}

/// The `expected-cost` relation in the only pair, or the reasons it was dropped or blocked.
fn cost(finding: &Value) -> (String, Vec<String>) {
    let pair = &finding["pairs"][0];
    let reasons = |v: &Value| -> Vec<String> {
        v["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_str().unwrap().to_string())
            .collect()
    };
    if let Some(m) = pair["measures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["measure_id"] == "expected-cost")
    {
        return (m["relation"].as_str().unwrap().to_string(), reasons(m));
    }
    let dropped = pair["dropped_measures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["measure_id"] == "expected-cost")
        .unwrap_or_else(|| panic!("expected-cost is in neither list: {pair}"));
    ("dropped".to_string(), reasons(dropped))
}

#[test]
fn a_confidence_equal_to_the_minimum_is_estimated_and_one_below_is_not() {
    // Both sets state `medium` for expected-cost; `contested` is `drop_measure` in this world.
    let w = world("boundary-confidence");
    rewrite_contract(&w, |c| {
        c["measures"][0]["minimum_confidence"] = json!("medium")
    });
    assert_eq!(cost(&compare(&w)), ("better".to_string(), vec![]));
    rewrite_contract(&w, |c| {
        c["measures"][0]["minimum_confidence"] = json!("high")
    });
    assert_eq!(
        cost(&compare(&w)),
        (
            "dropped".to_string(),
            vec!["below_minimum_confidence".to_string()]
        )
    );
}

#[test]
fn an_evidence_cutoff_equal_to_stale_before_is_not_stale_and_one_second_earlier_is() {
    // The expected-cost entries carry the cutoff 2026-10-21T10:00:00Z; `stale` is `block`.
    let w = world("boundary-stale");
    rewrite_contract(&w, |c| c["stale_before"] = json!("2026-10-21T10:00:00Z"));
    assert_eq!(cost(&compare(&w)), ("better".to_string(), vec![]));
    rewrite_contract(&w, |c| c["stale_before"] = json!("2026-10-21T10:00:01Z"));
    let finding = compare(&w);
    assert_eq!(finding["pairs"][0]["outcome"], "blocked");
    assert_eq!(finding["pairs"][0]["blocked_reasons"], json!(["stale"]));
    assert_eq!(cost(&finding).0, "blocked");
}

#[test]
fn intervals_that_touch_overlap_and_intervals_disjoint_by_one_dominate() {
    let w = world("boundary-interval");
    rewrite_contract(&w, |c| c["uncertainty"] = json!("interval_robust"));
    // alpha.one is [5, 12]; alpha.two is [12, 30] (touching), then [13, 30] (disjoint by one).
    edit_json(&w.sets[0], |set| {
        set["estimates"][0]["interval"]["low"] = json!({"quantity": 5});
        set["estimates"][0]["interval"]["high"] = json!({"quantity": 12});
    });
    for (low, relation, outcome) in [
        (12, "overlap", "incomparable"),
        (13, "better", "a_dominates_b"),
    ] {
        edit_json(&w.sets[1], |set| {
            set["estimates"][0]["interval"]["low"] = json!({"quantity": low});
            set["estimates"][0]["interval"]["high"] = json!({"quantity": 30});
        });
        let finding = compare(&w);
        assert_eq!(cost(&finding), (relation.to_string(), vec![]), "low {low}");
        assert_eq!(finding["pairs"][0]["outcome"], outcome, "low {low}");
    }
}
