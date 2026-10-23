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

// ---------------------------------------------------------------------------------------------
// A real on-disk world: packets, decisions, an evaluation basis derived in-process, an assessment.
// ---------------------------------------------------------------------------------------------

use proposal_evaluation::sha256_hex;
use std::path::{Path, PathBuf};

pub fn fresh_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("portfolio-fixtures")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::canonicalize(&dir).unwrap()
}

pub fn s(path: &Path) -> String {
    path.to_str().unwrap().to_string()
}

/// Write `value` as pretty JSON plus a newline; returns the sha256 of the bytes written.
pub fn write_json(path: &Path, value: &Value) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap();
    text.push('\n');
    write_text(path, &text)
}

pub fn write_text(path: &Path, text: &str) -> String {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, text).unwrap();
    sha256_hex(text.as_bytes())
}

pub fn file_ref(path: &Path, sha: &str) -> Value {
    json!({"path": s(path), "sha256": sha})
}

pub fn packet_json(id: &str, state: &str, relationships: Value) -> Value {
    json!({
        "schema_version": 1, "proposal_id": id, "family_id": "fam-one", "title": id,
        "lifecycle_state": state, "narrative": {"path": "narrative.md"},
        "origin_refs": [{"kind": "idea", "path": "ideas/x.md"}],
        "placement": {"state": "probable", "claim": "fits"},
        "uncertainty": {"state": "resolved", "reopening_conditions": []},
        "authority": {"decision_owner": "owner", "implementation_authorized": false},
        "relationships": relationships
    })
}

pub fn relationship(kind: &str, target: &str) -> Value {
    json!({"type": kind, "target_id": target, "rationale": "because", "causal": true})
}

/// A decision of the given class (`version` 1 or 2) for `id`.
pub fn decision_json(id: &str, version: u64) -> Value {
    let mut d = json!({
        "schema_version": version, "proposal_id": id,
        "source": {"repository_revision": "0123456789abcdef0123456789abcdef01234567",
                   "lifecycle_state": "evaluated", "placement_state": "probable"},
        "decision": {"disposition": "defer_for_dogfooding", "lifecycle_state": "decided",
                     "placement_state": "probable", "placement_claim": "c", "rationale": "r",
                     "reopening_conditions": ["later"]},
        "authority": {"decision_owner": "owner", "exercised_by": "operator", "evidence": "note"},
        "constraints": {"permanent_architecture_settled": false, "roadmap_priority_changed": false,
                        "implementation_authorized": false}
    });
    if version == 2 {
        d["supersedes"] = Value::Null;
    }
    d
}

pub fn assessment_json(purpose: &str, decision_id: &str, evaluation: Value) -> Value {
    json!({
        "schema": "decision-readiness-assessment-v1", "purpose": purpose,
        "assessment_id": "assess-1", "issued_at": "2026-10-23T09:00:00Z",
        "assessor": {"name": "a", "role": "r", "session": "s", "model_requested": "m",
                     "model_observed": "m", "authority_ref": "ref"},
        "decision": {"id": decision_id, "owner_role": "owner"},
        "profile": {"id": "profile-1", "version": 1, "sha256": SHA_A},
        "evaluation": evaluation, "verdict": "ready",
        "requirements": [], "missing": [], "indeterminate": [], "profile_gaps": [], "evidence": [],
        "freshness_basis": {
            "observation": {"battery_file_sha256": SHA_A, "battery_sha256": SHA_A,
                            "observation_sha256": SHA_A, "started_at": "2026-10-23T08:00:00Z",
                            "finished_at": "2026-10-23T08:30:00Z"},
            "claim_reliance": [], "note": "n"
        },
        "limits": []
    })
}

fn basis(n: u8) -> Value {
    json!({"claim_id": format!("claim-{n}"), "claim_revision_id": format!("rev-{n}"),
           "claim_revision_sha256": format!("{n:x}").repeat(64)})
}

/// A valid estimate set for `id` bound to the packet at `packet`.
pub fn estimate_set_json(id: &str, packet: &Value, projection: &Value, cost: i64) -> Value {
    json!({
        "schema": "proposal-evaluation-estimate-set-v1", "estimate_set_id": format!("set-{id}"),
        "proposal": {"proposal_id": id, "packet": packet},
        "estimator": {"role": "evaluator", "identity": "tester"},
        "issued_at": "2026-10-22T12:00:00Z", "evidence_cutoff": "2026-10-22T10:00:00Z",
        "claim_projection": projection,
        "estimates": [
            {"status": "estimated", "measure_id": "expected-cost",
             "measure": {"definition": "engineering days",
                         "scale": {"kind": "quantity", "unit": "day", "exponent": 0},
                         "direction": "lower_is_better", "scope": "this proposal"},
             "evidence_cutoff": "2026-10-21T10:00:00Z", "baseline": {"quantity": 10},
             "basis": [basis(1)], "limitations": ["synthetic"], "value": {"quantity": cost},
             "interval": {"low": {"quantity": cost - 1}, "high": {"quantity": cost + 1}},
             "confidence": "medium"},
            {"status": "contested", "measure_id": "regression_risk",
             "measure": {"definition": "chance of regression",
                         "scale": {"kind": "ordinal", "levels": ["low", "medium", "high"]},
                         "direction": "lower_is_better", "scope": "this proposal"},
             "evidence_cutoff": null, "baseline": null, "basis": [], "limitations": [],
             "competing": [{"value": {"level": "low"}, "basis": [basis(2)]},
                           {"value": {"level": "high"}, "basis": [basis(3)]}]},
            {"status": "not_estimated", "measure_id": "unlock.value",
             "measure": {"definition": "unlocked work",
                         "scale": {"kind": "quantity", "unit": "item", "exponent": -1},
                         "direction": "higher_is_better", "scope": "portfolio"},
             "evidence_cutoff": null, "baseline": null, "basis": [], "limitations": [],
             "reason": "no evidence yet"}
        ],
        "limitations": ["synthetic fixture"],
        "authority": {"readiness_judgment_issued": false, "priority_authorized": false,
                      "acceptance_authorized": false, "implementation_authorized": false}
    })
}

pub struct World {
    pub dir: PathBuf,
    /// A valid, fully bound portfolio record: three proposals, a comparison and an estimate set
    /// in `evaluation_basis`, one live assessment, v1 and v2 decisions bound.
    pub doc: Value,
    pub paths: std::collections::BTreeMap<&'static str, PathBuf>,
}

impl World {
    pub fn path(&self, key: &str) -> &Path {
        self.paths
            .get(key)
            .unwrap_or_else(|| panic!("no fixture {key}"))
    }

    pub fn sha(&self, key: &str) -> String {
        sha256_hex(&std::fs::read(self.path(key)).unwrap())
    }

    pub fn reference(&self, key: &str) -> Value {
        file_ref(self.path(key), &self.sha(key))
    }

    /// The bytes of the record, as `check` reads them.
    pub fn record(&self) -> Vec<u8> {
        bytes(&self.doc)
    }

    /// Overwrite fixture `key` with `text`, leaving every stored digest stale.
    pub fn overwrite(&self, key: &str, text: &str) {
        std::fs::write(self.path(key), text).unwrap();
    }
}

/// `old_beta`: beta-two's estimate set and the contract bind an older packet revision than the
/// one the portfolio binds.
pub fn world(name: &str, old_beta: bool) -> World {
    let dir = fresh_dir(name);
    let mut paths = std::collections::BTreeMap::new();
    let mut put = |key: &'static str, path: PathBuf| {
        paths.insert(key, path);
    };
    let rel_a = json!([relationship("enables", "beta-two")]);
    let rel_b = json!([relationship("depends_on", "alpha-one")]);
    let a_packet = dir.join("alpha-one/packet.json");
    let b_packet = dir.join("beta-two/packet.json");
    let g_packet = dir.join("gamma-three/packet.json");
    let sha_a = write_json(&a_packet, &packet_json("alpha-one", "decided", rel_a));
    let sha_b = write_json(
        &b_packet,
        &packet_json("beta-two", "decided", rel_b.clone()),
    );
    let sha_g = write_json(
        &g_packet,
        &packet_json("gamma-three", "evaluated", json!([])),
    );
    let a_decision = dir.join("alpha-one/decision.json");
    let b_decision = dir.join("beta-two/decision.json");
    let sha_ad = write_json(&a_decision, &decision_json("alpha-one", 1));
    let sha_bd = write_json(&b_decision, &decision_json("beta-two", 2));
    // The packet an old estimate set was written against.
    let b_old = dir.join("beta-two/packet-old.json");
    let sha_b_old = write_json(&b_old, &packet_json("beta-two", "evaluated", rel_b));
    let (b_bound, b_bound_sha) = if old_beta {
        (&b_old, sha_b_old.as_str())
    } else {
        (&b_packet, sha_b.as_str())
    };

    let mut sets = Vec::new();
    for (id, packet, packet_sha, cost) in [
        ("alpha-one", &a_packet, sha_a.as_str(), 8),
        ("beta-two", b_bound, b_bound_sha, 20),
    ] {
        let projection = dir.join(id).join("projection.json");
        let element = |n: u8| {
            json!({"revision_id": format!("rev-{n}"), "revision_sha256": format!("{n:x}").repeat(64),
                   "claim": {"id": format!("claim-{n}"), "profile": "proposal-research-v1"}})
        };
        let proj_sha = write_json(
            &projection,
            &json!({"schema_version": 1, "context_kind": "rust_reviewer_exact_claim_projection_v1",
                    "selection_metadata": [element(1), element(2), element(3)]}),
        );
        let set = estimate_set_json(
            id,
            &file_ref(packet, packet_sha),
            &file_ref(&projection, &proj_sha),
            cost,
        );
        let path = dir.join(id).join("set.json");
        write_json(&path, &set);
        sets.push(path);
    }
    let contract_path = dir.join("contract.json");
    write_json(
        &contract_path,
        &json!({
            "schema": "proposal-comparison-contract-v1", "contract_id": "contract-1", "version": 1,
            "selecting_authority": {"decision_owner": "portfolio owner", "exercised_by": "tester",
                                    "evidence": "fixture"},
            "decision_scope": "fixture scope",
            "proposals": [{"proposal_id": "alpha-one", "packet_sha256": sha_a},
                          {"proposal_id": "beta-two", "packet_sha256": b_bound_sha}],
            "measures": [
                {"measure_id": "expected-cost", "definition": "engineering days",
                 "scale": {"kind": "quantity", "unit": "day", "exponent": 0},
                 "direction": "lower_is_better", "scope": "this proposal", "minimum_confidence": "low"},
                {"measure_id": "regression_risk", "definition": "chance of regression",
                 "scale": {"kind": "ordinal", "levels": ["low", "medium", "high"]},
                 "direction": "lower_is_better", "scope": "this proposal", "minimum_confidence": "low"}
            ],
            "excluded_concerns": [{"concern": "morale", "reason": "not measured"}],
            "treatment": {"missing": "block", "contested": "drop_measure", "stale": "block"},
            "uncertainty": "point", "evidence_cutoff": "2026-10-22T10:00:00Z",
            "stale_before": "2026-10-01T00:00:00Z",
            "authority": {"priority_authorized": false, "acceptance_authorized": false}
        }),
    );
    let finding = proposal_evaluation::compare::compare_paths(
        &s(&contract_path),
        &sets.iter().map(|p| s(p)).collect::<Vec<_>>(),
        "2026-10-22T13:00:00Z",
    )
    .unwrap_or_else(|f| panic!("derive failed: {f} {:?}", f.errors()));
    let comparison = dir.join("comparison.json");
    write_json(&comparison, &serde_json::to_value(&finding).unwrap());

    let handoff = dir.join("handoff.json");
    write_json(&handoff, &json!({"strategic": "content is never parsed"}));
    let evaluation = dir.join("readiness-evaluation.json");
    let sha_ev = write_json(&evaluation, &json!({"opaque": true}));
    let assessment = dir.join("assessment.json");
    write_json(
        &assessment,
        &assessment_json("live", "decision-1", file_ref(&evaluation, &sha_ev)),
    );

    for (key, path) in [
        ("handoff", handoff),
        ("a_packet", a_packet),
        ("b_packet", b_packet),
        ("g_packet", g_packet),
        ("a_decision", a_decision),
        ("b_decision", b_decision),
        ("b_packet_old", b_old),
        ("set_alpha", sets[0].clone()),
        ("set_beta", sets[1].clone()),
        ("contract", contract_path),
        ("comparison", comparison),
        ("evaluation", evaluation),
        ("assessment", assessment),
    ] {
        put(key, path);
    }
    let mut w = World {
        dir,
        doc: Value::Null,
        paths,
    };
    let _ = (sha_g, sha_ad, sha_bd);
    let member = |id: &str, pk: &str, dk: Option<&str>| {
        json!({"proposal_id": id, "packet": w.reference(pk),
               "decision": dk.map(|k| w.reference(k))})
    };
    let mut doc = sample();
    doc["strategic_basis"] = json!({"handoff": w.reference("handoff")});
    doc["proposal_set"] = json!([
        member("alpha-one", "a_packet", Some("a_decision")),
        member("beta-two", "b_packet", Some("b_decision")),
        member("gamma-three", "g_packet", None)
    ]);
    doc["evaluation_basis"] = json!([
        {"kind": "comparison", "path": s(w.path("comparison")), "sha256": w.sha("comparison")},
        {"kind": "estimate_set", "path": s(w.path("set_alpha")), "sha256": w.sha("set_alpha")}
    ]);
    doc["readiness_basis"] = json!([
        {"proposal_id": "alpha-one", "decision_id": "decision-1", "assessment": w.reference("assessment")}
    ]);
    doc["limitations"] = json!([
        "strategic_basis_unverified: the handoff is hashed, not read",
        "relationship_graph_revision_unverified: packet bytes are not matched to the revision",
        "readiness_decision_fit_unverified: the author judges the fit"
    ]);
    w.doc = doc;
    w
}
