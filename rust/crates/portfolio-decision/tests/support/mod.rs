//! Shared fixtures. Records are built as `serde_json::Value`s and edited by dotted path so a
//! test states exactly the one thing it breaks.
#![allow(dead_code)]

use serde_json::{Value, json};

pub const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

pub const IDS: [&str; 3] = ["alpha-one", "beta-two", "gamma-three"];

/// A structurally valid record whose pointers are well formed but point nowhere.
pub fn sample() -> Value {
    json!({
        "schema": "portfolio-decision-v1",
        "portfolio_id": "pf-main",
        "revision": 1,
        "supersedes": null,
        "decided_at": "2026-10-23T12:00:00Z",
        "authority": {"decision_owner": "owner", "exercised_by": "operator", "evidence": "session note"},
        "strategic_basis": {"handoff": {"path": "/x/handoff.json", "sha256": SHA_A}},
        "proposal_set": IDS.iter().map(|id| json!({
            "proposal_id": id,
            "packet": {"path": format!("/x/{id}/packet.json"), "sha256": SHA_A},
            "decision": null
        })).collect::<Vec<_>>(),
        "relationship_graph": {"repository_revision": "0123456789abcdef0123456789abcdef01234567", "source": "packet_relationships"},
        "evaluation_basis": [],
        "readiness_basis": [],
        "cross_proposal_analysis": {"unlock": [], "mutually_exclusive_under_basis": []},
        "dispositions": [
            {"disposition": "prioritize", "proposal_id": "alpha-one", "rationale": "first", "rank": 1},
            {"disposition": "sequence", "proposal_id": "beta-two", "rationale": "after alpha", "after": ["alpha-one"]},
            {"disposition": "defer", "proposal_id": "gamma-three", "rationale": "not now", "revisit_when": ["capacity frees"]}
        ],
        "non_effects": {
            "proposal_lifecycle_mutated": false, "proposal_rejected": false,
            "relationship_graph_mutated": false, "implementation_authorized": false,
            "schedule_executed": false
        },
        "limitations": [
            "strategic_basis_unverified: the handoff is hashed, not read",
            "relationship_graph_revision_unverified: packet bytes are not matched to the revision",
            "no_evaluation_basis: no comparison or estimate set was consulted"
        ]
    })
}

/// Set `value` at a dotted `path` (array indexes are numbers). Panics when the path is absent,
/// so a typo in a test cannot silently edit nothing.
pub fn set(doc: &mut Value, path: &str, value: Value) {
    let mut cur = doc;
    let parts: Vec<&str> = path.split('.').collect();
    for (i, part) in parts.iter().enumerate() {
        let last = i + 1 == parts.len();
        let next = match cur {
            Value::Array(a) => a.get_mut(part.parse::<usize>().expect("array index")),
            Value::Object(o) => o.get_mut(*part),
            _ => None,
        }
        .unwrap_or_else(|| panic!("no such path {path} at {part}"));
        if last {
            *next = value;
            return;
        }
        cur = next;
    }
}

/// Remove the key at a dotted `path`.
pub fn remove(doc: &mut Value, path: &str) {
    let (parent, key) = path.rsplit_once('.').unwrap_or(("", path));
    let target = if parent.is_empty() {
        doc
    } else {
        pointer_mut(doc, parent)
    };
    target
        .as_object_mut()
        .expect("object")
        .remove(key)
        .unwrap_or_else(|| panic!("no key {path}"));
}

pub fn pointer_mut<'a>(doc: &'a mut Value, path: &str) -> &'a mut Value {
    let mut cur = doc;
    for part in path.split('.') {
        cur = match cur {
            Value::Array(a) => a
                .get_mut(part.parse::<usize>().expect("index"))
                .expect("idx"),
            Value::Object(o) => o.get_mut(part).unwrap_or_else(|| panic!("no key {part}")),
            _ => panic!("not a container at {part}"),
        };
    }
    cur
}

pub fn bytes(doc: &Value) -> Vec<u8> {
    serde_json::to_vec_pretty(doc).unwrap()
}

/// `(code, at)` pairs of a report, in order.
pub fn codes_of(report: &portfolio_decision::CheckReport) -> Vec<(String, String)> {
    report
        .errors
        .iter()
        .map(|e| (e.code.clone(), e.at.clone()))
        .collect()
}
