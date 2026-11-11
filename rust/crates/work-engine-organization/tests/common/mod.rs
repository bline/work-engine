//! Shared helpers for the integration tests. Inputs are built with `serde_json::json!` and handed
//! to the crate as bytes, as authored files are.

#![allow(dead_code)]

use serde_json::{Value, json};
use work_engine_organization::{
    DerivationResult, OrganizationCandidate, OrganizationalRequirements, Profile, derive,
};

pub const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

pub fn normative() -> Value {
    json!({
        "continuity": "fresh_initial",
        "mutation": "none",
        "capabilities": [],
        "integrity": "verified_artifacts",
        "evidenceCustody": "execution_evidence_custody",
        "observerClass": "independent_execution_evidence"
    })
}

pub fn obligation(id: &str, digest: &str) -> Value {
    json!({
        "id": id,
        "kind": "candidate_review",
        "subject": "campaign_candidate",
        "resultContract": {"id": "implementation-review", "version": 1},
        "scope": {"digest": digest}
    })
}

pub fn condition(id: &str, obligation: &str, boundary: &str, class: &str, acceptor: &str) -> Value {
    json!({
        "id": id,
        "obligation": obligation,
        "consumptionBoundary": boundary,
        "consumerClass": class,
        "proposition": format!("proposition of {id}"),
        "coveredState": "review-result",
        "normative": normative(),
        "acceptor": acceptor,
        "unestablishedRoute": "hold-consumption-for-owner-disposition"
    })
}

pub fn authority(
    id: &str,
    principal: &str,
    permission: &str,
    boundaries: Option<&[&str]>,
) -> Value {
    let scope = match boundaries {
        Some(list) => json!({ "boundaries": list }),
        None => json!({}),
    };
    json!({"id": id, "principal": principal, "permission": permission, "scope": scope})
}

pub fn document(obligations: Vec<Value>, conditions: Vec<Value>, authorities: Vec<Value>) -> Value {
    json!({
        "problemScope": {"runId": "run-1", "planLineage": "plan-a", "sliceNumber": 1},
        "obligations": obligations,
        "acceptanceConditions": conditions,
        "requiredAuthorities": authorities,
        "organizationPolicy": {"mode": "fixed", "profile": "fixed-slice-review-v1"}
    })
}

/// One obligation `ob-a`; a builder-projection and a campaign-terminalization condition accepted
/// by `owner-a`; `auth-1` is the only entry that states the acceptor dependency (whole scope);
/// `auth-extra` is an entry the closure does not need.
pub fn base() -> Value {
    document(
        vec![obligation("ob-a", DIGEST_A)],
        vec![
            condition(
                "cond-builder",
                "ob-a",
                "builder_projection",
                "builder",
                "owner-a",
            ),
            condition(
                "cond-campaign",
                "ob-a",
                "campaign_terminalization",
                "campaign",
                "owner-a",
            ),
        ],
        vec![
            authority("auth-1", "owner-a", "accept_claim", None),
            authority("auth-extra", "observer-1", "audit_read", None),
        ],
    )
}

pub fn bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

pub fn parse(value: &Value) -> OrganizationalRequirements {
    OrganizationalRequirements::from_json_bytes(&bytes(value))
        .unwrap_or_else(|error| panic!("input refused: {error}"))
}

pub fn run(value: &Value) -> DerivationResult {
    derive(&parse(value), Profile::FixedSliceReviewV1)
}

pub fn candidate(value: &Value) -> OrganizationCandidate {
    let mut result = run(value);
    assert!(
        result.unresolved.is_empty(),
        "unexpected unresolved: {:?}",
        result.unresolved
    );
    assert_eq!(result.candidates.len(), 1);
    result.candidates.remove(0)
}

/// `kind|from|to|condition` with absent parts empty.
pub fn relationship_keys(candidate: &OrganizationCandidate) -> Vec<String> {
    candidate
        .relationships()
        .iter()
        .map(|r| {
            format!(
                "{}|{}|{}|{}",
                r.kind.as_str(),
                r.from,
                r.to.as_deref().unwrap_or(""),
                r.condition.as_ref().map_or("", |c| c.as_str())
            )
        })
        .collect()
}

/// A vantage whose two conditions state different normative content (F4 combination).
pub fn two_condition_vantage() -> Value {
    let mut first = condition("c1", "ob-a", "builder_projection", "builder", "owner-a");
    first["normative"]["independence"] = json!({"from": ["state.builder_context"]});
    first["realizationConstraints"] = json!([
        {"dimension": "provider_family", "allowed": ["anthropic"]},
        {"dimension": "execution_confinement", "allowed": ["guarded_sandbox"]}
    ]);
    let mut second = condition(
        "c2",
        "ob-a",
        "campaign_terminalization",
        "campaign",
        "owner-a",
    );
    second["realizationConstraints"] =
        json!([{"dimension": "provider_family", "allowed": ["anthropic"]}]);
    document(
        vec![obligation("ob-a", DIGEST_A)],
        vec![first, second],
        vec![authority("auth-1", "owner-a", "accept_claim", None)],
    )
}
