//! Preconditions of the admission tests: the shape of the base candidate they rely on.

use serde_json::json;
use work_engine_organization::{
    EnforceStatus, OrganizationalRequirements, Profile, derive, enforceability,
};

#[test]
fn base_candidate_shape_that_admission_tests_assume() {
    let normative = json!({"continuity":"fresh_initial","mutation":"none","capabilities":[],"integrity":"verified_artifacts","evidenceCustody":"execution_evidence_custody","observerClass":"independent_execution_evidence"});
    let cond = |id: &str, b: &str, c: &str| json!({"id":id,"obligation":"ob-a","consumptionBoundary":b,"consumerClass":c,"proposition":"p","coveredState":"review-result","normative":normative,"acceptor":"owner-a","unestablishedRoute":"hold-consumption-for-owner-disposition"});
    let doc = json!({
        "problemScope": {"runId": "run-1", "planLineage": "plan-a", "sliceNumber": 1},
        "obligations": [{"id":"ob-a","kind":"candidate_review","subject":"campaign_candidate","resultContract":{"id":"implementation-review","version":1},"scope":{"digest":"a".repeat(64)}}],
        "acceptanceConditions": [cond("cond-builder","builder_projection","builder"), cond("cond-campaign","campaign_terminalization","campaign")],
        "requiredAuthorities": [{"id":"auth-1","principal":"owner-a","permission":"accept_claim","scope":{}}],
        "organizationPolicy": {"mode": "fixed", "profile": "fixed-slice-review-v1"}
    });
    let r =
        OrganizationalRequirements::from_json_bytes(&serde_json::to_vec(&doc).unwrap()).unwrap();
    let d = derive(&r, Profile::FixedSliceReviewV1);
    assert!(d.unresolved.is_empty());
    let c = &d.candidates[0];
    let separation: Vec<_> = c
        .separation_requirements()
        .iter()
        .map(|s| {
            (
                s.condition.as_str().to_owned(),
                s.acceptor.clone(),
                s.vantages.clone(),
            )
        })
        .collect();
    let vantages = vec!["vantage-builder".to_owned(), "vantage:ob-a".to_owned()];
    assert_eq!(
        separation,
        vec![
            (
                "cond-builder".into(),
                "principal:owner-a".into(),
                vantages.clone()
            ),
            ("cond-campaign".into(), "principal:owner-a".into(), vantages),
        ]
    );
    let components: Vec<_> = c
        .component_binding_requirements()
        .iter()
        .map(|b| (b.component.clone(), b.permissions.clone()))
        .collect();
    assert_eq!(
        components,
        vec![
            (
                "component:campaign".into(),
                vec!["terminalize_campaign".into()]
            ),
            (
                "component:episode_host".into(),
                vec!["read_episode".into(), "write_episode".into()]
            ),
        ]
    );
    let none_rows: Vec<_> = enforceability(c, Profile::FixedSliceReviewV1)
        .rows
        .into_iter()
        .filter(|row| row.status == EnforceStatus::None)
        .map(|row| format!("{}/{}", row.scope, row.property))
        .collect();
    assert_eq!(
        none_rows,
        vec!["cond-builder/separation", "cond-campaign/separation"]
    );
}
