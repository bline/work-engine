//! E2 review fix P2-1: `check_comparison_bytes` resolves at the path contract 5 names.

mod support;
use std::path::PathBuf;
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

#[test]
fn check_comparison_bytes_resolves_at_the_path_the_contract_names() {
    // Contract 5: `proposal_evaluation::check::check_comparison_bytes`, and the crate root.
    let w = world("ptr-api-path");
    let finding = derive_to(&w, "finding.json");
    let bytes = std::fs::read(&finding).unwrap();
    let by_contract_path = proposal_evaluation::check::check_comparison_bytes(&bytes, false)
        .expect("a derived finding checks");
    assert!(by_contract_path.valid, "{:?}", by_contract_path.errors);
    let at_root: fn(&[u8], bool) -> _ = proposal_evaluation::check_comparison_bytes;
    let at_check: fn(&[u8], bool) -> _ = proposal_evaluation::check::check_comparison_bytes;
    assert_eq!(
        at_root as usize, at_check as usize,
        "one function, two paths"
    );
}
