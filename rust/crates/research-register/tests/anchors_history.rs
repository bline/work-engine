//! Pinned historical regression: `anchors` at 29 commits of this repository against the
//! projection of `register.py anchors` output committed under `tests/fixtures/anchors/`
//! (regenerate with `project.py` there). The enclosing repository must be present with full
//! history: an unavailable or shallow repository fails this test, it is never skipped.

use research_register::{GitObjects, Outcome, PinnedSnapshot, anchors, validate};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const FIELDS: &[&str] = &[
    "decision_id",
    "class",
    "error",
    "owner_review_required",
    "owner_review_reasons",
    "revision_commit",
    "before",
    "after",
    "candidates",
    "document_at_target",
];

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/anchors")
}

fn project(entry: &Value) -> Value {
    let mut row = serde_json::Map::new();
    for f in FIELDS {
        row.insert((*f).into(), entry.get(*f).cloned().unwrap_or(Value::Null));
    }
    for (k, ctx) in [
        ("context_before.section_chain", "context_before"),
        ("context_after.section_chain", "context_after"),
    ] {
        let chain = entry
            .get(ctx)
            .and_then(|c| c.get("section_chain"))
            .cloned()
            .unwrap_or(Value::Null);
        row.insert(k.into(), chain);
    }
    Value::Object(row)
}

#[test]
fn historical_anchors_equal_register_py() {
    let started = std::time::Instant::now();
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let targets = std::fs::read_to_string(fixtures().join("targets.txt")).unwrap();
    let targets: Vec<&str> = targets.split_whitespace().collect();
    assert_eq!(targets.len(), 29, "pinned target count");
    let (mut rows, mut mismatches) = (0, Vec::new());
    for commit in &targets {
        let objects = GitObjects::open(&repo).unwrap_or_else(|e| {
            panic!(
                "the enclosing repository {} is required (full history): {e}",
                repo.display()
            )
        });
        let want: Value = serde_json::from_str(
            &std::fs::read_to_string(fixtures().join(format!("{commit}.json"))).unwrap(),
        )
        .unwrap();
        let register = want["register_path"].as_str().unwrap();
        let snapshot = PinnedSnapshot::build(objects, commit, register)
            .unwrap_or_else(|e| panic!("snapshot at {commit}: {e}"));
        let valid = match validate(&snapshot).unwrap_or_else(|e| panic!("validate {commit}: {e}")) {
            Outcome::Valid(v) => v,
            Outcome::Invalid(v) => panic!("register invalid at {commit}: {v:?}"),
        };
        let report = anchors(&valid, &snapshot).unwrap_or_else(|e| panic!("anchors {commit}: {e}"));
        assert_eq!(report.pinned_commit.to_string(), *commit);
        let got: Vec<Value> = report.entries.iter().map(|e| project(e.json())).collect();
        let want_entries = want["entries"].as_array().unwrap();
        assert_eq!(got.len(), want_entries.len(), "entry count at {commit}");
        for (g, w) in got.iter().zip(want_entries) {
            rows += 1;
            if g != w {
                mismatches.push(json!({"commit": commit, "want": w, "got": g}));
            }
        }
    }
    eprintln!(
        "anchors history: {} targets, {rows} rows, {} mismatches, {:.1?}",
        targets.len(),
        mismatches.len(),
        started.elapsed()
    );
    assert_eq!(rows, 319, "pinned row count");
    assert!(
        mismatches.is_empty(),
        "{} mismatches, first: {:#}",
        mismatches.len(),
        mismatches[0]
    );
}
