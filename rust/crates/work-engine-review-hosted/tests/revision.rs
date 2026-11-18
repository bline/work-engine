//! Outcomes 1 and 2: the requirements revision, the hosted claim, preparation and refusal.

mod common;

use serde_json::Value;
use std::collections::BTreeSet;
use work_engine_review_hosted::revision::{
    self, CLAUDE_TOOLS, FROZEN_REQUIREMENTS, Profile, REVISION_ID, REVISION_R2_ID,
    REVISION_R2_REQUIREMENTS, REVISION_REQUIREMENTS, frozen_world, frozen_world_for, hosted_claim,
    prepare_world, prepare_world_for,
};
use work_engine_review_hosted::task::Limits;
use work_engine_review_hosted::{Refusal, inputs, sha256_hex};
use work_engine_review_preparation::{BlockerClass, Disposition, MutationClaim};

/// The frozen fixture stays byte-identical (it is consumed, never modified).
const FROZEN_SHA256: &str = "7085318950aceb6f7071d29dec83e13da610b20d110f435e3399c9de57d3b292";

/// Every JSON pointer at which `a` and `b` differ (leaf-wise; a member present on one side only
/// is reported at that member).
fn differing(a: &Value, b: &Value, at: String, out: &mut BTreeSet<String>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for key in x.keys().chain(y.keys()).collect::<BTreeSet<_>>() {
                let here = format!("{at}/{key}");
                match (x.get(key), y.get(key)) {
                    (Some(p), Some(q)) => differing(p, q, here, out),
                    _ => {
                        out.insert(here);
                    }
                }
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (p, q)) in x.iter().zip(y).enumerate() {
                differing(p, q, format!("{at}/{i}"), out);
            }
        }
        _ if a != b => {
            out.insert(at);
        }
        _ => {}
    }
}

#[test]
fn revision_is_the_frozen_requirements_with_exactly_the_authorized_changes() {
    assert_eq!(sha256_hex(FROZEN_REQUIREMENTS), FROZEN_SHA256);
    let frozen: Value = serde_json::from_slice(FROZEN_REQUIREMENTS).unwrap();
    let revised: Value = serde_json::from_slice(REVISION_REQUIREMENTS).unwrap();
    let mut diff = BTreeSet::new();
    differing(&frozen, &revised, String::new(), &mut diff);
    let expected: BTreeSet<String> = [
        "/problemScope/runId",
        "/acceptanceConditions/0/realizationConstraints",
        "/acceptanceConditions/0/proposition",
        "/acceptanceConditions/1/realizationConstraints",
        "/acceptanceConditions/1/proposition",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    assert_eq!(diff, expected);

    // The removed members are exactly the two realization-class constraints.
    for condition in frozen["acceptanceConditions"].as_array().unwrap() {
        let dims: Vec<&str> = condition["realizationConstraints"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["dimension"].as_str().unwrap())
            .collect();
        assert_eq!(dims, ["provider_family", "execution_confinement"]);
    }
    // The derivation as DERIVATION.md states it reproduces the revision.
    let mut derived = frozen.clone();
    derived["problemScope"]["runId"] = revised["problemScope"]["runId"].clone();
    for (i, c) in derived["acceptanceConditions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        c.as_object_mut().unwrap().remove("realizationConstraints");
        c["proposition"] = revised["acceptanceConditions"][i]["proposition"].clone();
    }
    assert_eq!(derived, revised);

    // New run identity; truthful proposition naming the route and its limits.
    assert_eq!(
        revised["problemScope"]["runId"],
        "hosted-route-qualification-20261117"
    );
    assert_ne!(
        revised["problemScope"]["runId"],
        frozen["problemScope"]["runId"]
    );
    for c in revised["acceptanceConditions"].as_array().unwrap() {
        let p = c["proposition"].as_str().unwrap();
        for needle in [
            "Route qualification only",
            REVISION_ID,
            "work-engine.pi-analysis-hosted.v1",
            "gpt-6-luna",
            "openai-codex",
            "snapshot_list",
            "snapshot_read",
            "not a guarded Claude reviewer",
            "No qualification outcome establishes this condition",
            "consumable",
        ] {
            assert!(p.contains(needle), "{needle} missing from {p}");
        }
    }
}

#[test]
fn revision_compiles_with_the_unchanged_compiler() {
    let candidate = revision::candidate(REVISION_REQUIREMENTS).expect("one candidate");
    assert_eq!(
        candidate.problem_scope().run_id.as_str(),
        "hosted-route-qualification-20261117"
    );
    // The frozen requirements still compile too (unchanged).
    revision::candidate(FROZEN_REQUIREMENTS).expect("frozen candidate");
}

#[test]
fn hosted_claim_states_what_the_host_enforces() {
    let claim = hosted_claim();
    assert_eq!(claim.provider_family.as_deref(), Some("openai-codex"));
    assert_eq!(claim.execution_confinement, None);
    assert_eq!(claim.mutation, MutationClaim::ReadOnlyClaimed);
    assert_eq!(claim.granted_tools, ["snapshot_list", "snapshot_read"]);
    assert_eq!(claim.continuity.as_deref(), Some("fresh_initial"));
}

#[test]
fn frozen_candidate_with_the_hosted_claim_is_refused() {
    let subject = common::frozen_subject();
    let Err(Refusal::KnownIncompatibility(ids)) =
        prepare_world(FROZEN_REQUIREMENTS, &subject, "job-1")
    else {
        panic!("the frozen world must be refused");
    };
    // One provider-family incompatibility per condition; confinement is unstated, so it is
    // unestablished, not incompatible.
    assert_eq!(ids.len(), 2, "{ids:?}");
    assert!(
        ids.iter().all(|id| id.contains("provider_family")),
        "{ids:?}"
    );
    let world = frozen_world(&subject, "job-1").unwrap();
    assert_eq!(world.outcome, "refused");
    assert_eq!(world.known_incompatibilities, ids);
    assert_eq!(world.requirements_sha256, FROZEN_SHA256);
}

#[test]
fn frozen_candidate_writes_no_job_inputs() {
    let repo = common::repo();
    let out = repo.path().join("out");
    let refused = inputs::write(
        Profile::OpenaiCodex,
        FROZEN_REQUIREMENTS,
        &repo.path(),
        &common::frozen_subject(),
        "job-1",
        Limits {
            max_requests: 8,
            max_reads: 16,
        },
        &out,
    );
    assert!(
        matches!(refused, Err(Refusal::KnownIncompatibility(_))),
        "{refused:?}"
    );
    assert!(!out.exists(), "a refusal leaves no output");
}

#[test]
fn revision_world_prepares_and_freezes_with_every_blocker_kept() {
    let subject = common::frozen_subject();
    let frozen = prepare_world(REVISION_REQUIREMENTS, &subject, "job-7").unwrap();
    assert_eq!(frozen.transport().attempt_id, "job-7");
    assert_eq!(frozen.transport().generation, 1);
    let prep = frozen.preparation();
    assert_eq!(
        prep.model_context.obligation.scope_digest,
        subject.patch_identity
    );
    assert_eq!(
        prep.model_context.identity.run_id,
        "hosted-route-qualification-20261117"
    );
    assert!(!prep.blockers.is_empty());
    assert!(
        prep.blockers
            .iter()
            .all(|b| b.class == BlockerClass::Unestablished)
    );
    let evaluation = prep.realization_evaluation.as_ref().unwrap();
    for condition in &evaluation.conditions {
        for property in ["mutation", "capabilities"] {
            let check = condition
                .checks
                .iter()
                .find(|c| c.property == property)
                .unwrap();
            assert_eq!(check.disposition, Disposition::Unestablished, "{property}");
        }
        assert!(
            !condition
                .checks
                .iter()
                .any(|c| c.property.starts_with("realization:"))
        );
    }
    // Deterministic: the same inputs freeze to the same bytes.
    let again = prepare_world(REVISION_REQUIREMENTS, &subject, "job-7").unwrap();
    assert_eq!(frozen.contract_sha256(), again.contract_sha256());
    assert_eq!(
        sha256_hex(frozen.contract().as_bytes()),
        frozen.contract_sha256()
    );
    eprintln!(
        "revision contract: {} bytes, sha256 {}, {} blockers",
        frozen.contract().len(),
        frozen.contract_sha256(),
        prep.blockers.len()
    );
}

// ---- Revision r2 (operator decision D, 2026-11-18) and the claude-sdk profile ----

#[test]
fn revision_r2_is_the_frozen_requirements_without_only_execution_confinement() {
    let frozen: Value = serde_json::from_slice(FROZEN_REQUIREMENTS).unwrap();
    let r1: Value = serde_json::from_slice(REVISION_REQUIREMENTS).unwrap();
    let r2: Value = serde_json::from_slice(REVISION_R2_REQUIREMENTS).unwrap();
    let mut diff = BTreeSet::new();
    differing(&frozen, &r2, String::new(), &mut diff);
    let expected: BTreeSet<String> = [
        "/problemScope/runId",
        "/acceptanceConditions/0/realizationConstraints",
        "/acceptanceConditions/0/proposition",
        "/acceptanceConditions/1/realizationConstraints",
        "/acceptanceConditions/1/proposition",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    assert_eq!(diff, expected);
    // Each condition keeps exactly the frozen provider_family constraint, nothing else.
    for (i, condition) in r2["acceptanceConditions"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let frozen_constraints = frozen["acceptanceConditions"][i]["realizationConstraints"]
            .as_array()
            .unwrap();
        assert_eq!(
            condition["realizationConstraints"],
            Value::Array(vec![frozen_constraints[0].clone()])
        );
        assert_eq!(
            frozen_constraints[0]["dimension"], "provider_family",
            "the kept constraint"
        );
        assert_eq!(
            frozen_constraints[0]["allowed"],
            serde_json::json!(["anthropic"])
        );
        assert_eq!(frozen_constraints[1]["dimension"], "execution_confinement");
    }
    // The derivation as DERIVATION.md states it reproduces the revision.
    let mut derived = frozen.clone();
    derived["problemScope"]["runId"] = r2["problemScope"]["runId"].clone();
    for (i, c) in derived["acceptanceConditions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        c["realizationConstraints"]
            .as_array_mut()
            .unwrap()
            .retain(|r| r["dimension"] != "execution_confinement");
        c["proposition"] = r2["acceptanceConditions"][i]["proposition"].clone();
    }
    assert_eq!(derived, r2);
    // New run identity, distinct from the frozen one and from r1's.
    assert_eq!(
        r2["problemScope"]["runId"],
        "hosted-route-qualification-r2-20261118"
    );
    assert_ne!(r2["problemScope"]["runId"], frozen["problemScope"]["runId"]);
    assert_ne!(r2["problemScope"]["runId"], r1["problemScope"]["runId"]);
    // Truthful propositions naming the route and its limits.
    for c in r2["acceptanceConditions"].as_array().unwrap() {
        let p = c["proposition"].as_str().unwrap();
        for needle in [
            "Route qualification only",
            REVISION_R2_ID,
            "work-engine.pi-analysis-hosted.v1",
            "claude-haiku-5-5",
            "claude-sdk",
            "provider family anthropic",
            "mcp__pi__snapshot_list",
            "mcp__pi__snapshot_read",
            "execution confinement is not established",
            "No qualification outcome establishes this condition",
            "consumable",
        ] {
            assert!(p.contains(needle), "{needle} missing from {p}");
        }
    }
    // r1 stays in the tree unchanged (its own test pins its diff).
    assert_eq!(Profile::OpenaiCodex.revision().id, REVISION_ID);
    assert_eq!(Profile::ClaudeSdk.revision().id, REVISION_R2_ID);
}

#[test]
fn revision_r2_compiles_and_the_claude_sdk_claim_has_no_known_incompatibility() {
    let candidate = revision::candidate(REVISION_R2_REQUIREMENTS).expect("one candidate");
    assert_eq!(
        candidate.problem_scope().run_id.as_str(),
        "hosted-route-qualification-r2-20261118"
    );
    let subject = common::frozen_subject();
    // The unchanged `prepare`: no KnownIncompatibility; provider family claimed compatible.
    let prep = revision::preparation(Profile::ClaudeSdk, REVISION_R2_REQUIREMENTS, &subject)
        .expect("prepares");
    assert!(
        revision::known_incompatibilities(&prep).is_empty(),
        "{:?}",
        prep.blockers
    );
    assert!(
        prep.blockers
            .iter()
            .all(|b| b.class == BlockerClass::Unestablished)
    );
    let evaluation = prep.realization_evaluation.as_ref().unwrap();
    assert_eq!(evaluation.conditions.len(), 2);
    for condition in &evaluation.conditions {
        let family: Vec<_> = condition
            .checks
            .iter()
            .filter(|c| c.property.contains("provider_family"))
            .collect();
        assert_eq!(family.len(), 1, "{:?}", condition.checks);
        assert_eq!(family[0].disposition, Disposition::ClaimedCompatible);
        assert!(
            !condition
                .checks
                .iter()
                .any(|c| c.property.contains("execution_confinement"))
        );
        for property in ["mutation", "capabilities"] {
            let check = condition
                .checks
                .iter()
                .find(|c| c.property == property)
                .unwrap();
            assert_eq!(check.disposition, Disposition::Unestablished, "{property}");
        }
    }
    let frozen = prepare_world_for(
        Profile::ClaudeSdk,
        REVISION_R2_REQUIREMENTS,
        &subject,
        "job-7",
    )
    .expect("freezes");
    assert_eq!(
        frozen.preparation().model_context.identity.run_id,
        "hosted-route-qualification-r2-20261118"
    );
    assert_eq!(
        frozen.preparation().model_context.obligation.scope_digest,
        subject.patch_identity
    );
    // The openai-codex claim under r2 is known incompatible (that profile keeps r1).
    assert!(matches!(
        prepare_world_for(Profile::OpenaiCodex, REVISION_R2_REQUIREMENTS, &subject, "job-7"),
        Err(Refusal::KnownIncompatibility(ids)) if ids.len() == 2
    ));
}

#[test]
fn claude_sdk_claim_states_what_the_host_enforces() {
    let claim = Profile::ClaudeSdk.claim();
    assert_eq!(claim.provider_family.as_deref(), Some("anthropic"));
    assert_eq!(claim.execution_confinement, None);
    assert_eq!(claim.mutation, MutationClaim::ReadOnlyClaimed);
    assert_eq!(claim.granted_tools, CLAUDE_TOOLS);
    assert_eq!(claim.continuity.as_deref(), Some("fresh_initial"));
    assert_eq!(Profile::ClaudeSdk.provider(), "claude-sdk");
    assert_eq!(Profile::ClaudeSdk.model(), "claude-haiku-5-5");
    assert_eq!(Profile::OpenaiCodex.model(), "gpt-6-luna");
    // A closed set: only the two provider ids parse.
    assert_eq!(Profile::parse("claude-sdk"), Some(Profile::ClaudeSdk));
    assert_eq!(Profile::parse("openai-codex"), Some(Profile::OpenaiCodex));
    for other in ["anthropic", "openai", "Claude-SDK", "", "claude-sdk "] {
        assert_eq!(Profile::parse(other), None, "{other}");
    }
    assert!(serde_json::from_str::<Profile>("\"anthropic\"").is_err());
}

#[test]
fn frozen_candidate_on_the_claude_sdk_profile_stays_unmet_and_writes_no_job_inputs() {
    let subject = common::frozen_subject();
    let world = frozen_world_for(Profile::ClaudeSdk, &subject).unwrap();
    assert_eq!(world.outcome, "prepared_conditions_unmet");
    assert!(world.known_incompatibilities.is_empty());
    assert_eq!(world.unestablished_confinement.len(), 2, "{world:?}");
    assert!(
        world
            .unestablished_confinement
            .iter()
            .all(|id| id.ends_with(":realization:execution_confinement"))
    );
    assert_eq!(world.requirements_sha256, FROZEN_SHA256);
    // Passing the frozen requirements to the composition is refused, writing nothing.
    let repo = common::repo();
    let out = repo.path().join("out");
    let refused = inputs::write(
        Profile::ClaudeSdk,
        FROZEN_REQUIREMENTS,
        &repo.path(),
        &subject,
        "job-1",
        Limits {
            max_requests: 8,
            max_reads: 16,
        },
        &out,
    );
    assert!(
        matches!(refused, Err(Refusal::Requirements(_))),
        "{refused:?}"
    );
    assert!(!out.exists());
}
