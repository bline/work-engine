//! P1-AC0: two sibling crates that each carry their own `[workspace]` resolve as path
//! dependencies, and the constants the checker needs are reachable through their libraries.

#[test]
fn upstream_owners_are_reachable_as_path_dependencies() {
    let (comparison, estimate_set, assessment) = portfolio_decision::upstream_probe();
    assert_eq!(comparison, "proposal-evaluation-comparison-v1");
    assert_eq!(estimate_set, "proposal-evaluation-estimate-set-v1");
    assert_eq!(assessment, "decision-readiness-assessment-v1");
}
