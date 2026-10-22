//! `evaluate` outcomes (contract 3.6, 5.3), one fixture per status rule.
//!
//! Each test builds a fresh fixture tree, derives the plan with the tool, runs the unmodified
//! evidence battery over it, and evaluates the saved result.

mod support;
use support::*;

#[test]
fn base_case_evaluates_with_every_requirement_met_but_the_head_comparison() {
    // R-json predicate 6 compares the string "ok" with the `head` parameter (a commit id):
    // unmet `differs` by construction of the base fixture, which was built for `plan`.
    let case = base_case("eval-base");
    let observed = observe(&case, "base");
    let run = case.evaluate(&observed, &[]);
    assert_eq!(run.code, 0, "stderr: {}", run.stderr);
    let evaluation = run.out_json();
    assert!(evaluation.get("verdict").is_none());
    assert_eq!(evaluation["binding"]["git_root"], "verified");
    for requirement in evaluation["requirements"].as_array().unwrap() {
        if requirement["id"] == "R-json" {
            assert_eq!(requirement["status"], "unmet");
            assert_eq!(requirement["predicates"][5]["reason"], "differs");
            assert_eq!(requirement["predicates"][4]["status"], "met");
        } else if requirement["class"] == "mechanical" {
            assert_eq!(requirement["status"], "met", "{requirement}");
        } else {
            assert_eq!(requirement["status"], "indeterminate");
            assert_eq!(requirement["reason"], "attributed_judgment_required");
        }
    }
    assert_eq!(evaluation["counts"]["met"], 6);
    assert_eq!(evaluation["counts"]["unmet"], 1);
    assert_eq!(evaluation["counts"]["indeterminate"], 1);
}
