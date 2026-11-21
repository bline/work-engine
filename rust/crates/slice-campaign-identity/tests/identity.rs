//! slice-campaign's published identity rules over the frozen episode authority: the minted
//! episode id, the episode-within-scope predicate and the consumption-evidence mapping.
use review_episode_core::codec::parse_json;
use review_episode_core::identity::Identity;
use serde_json::{Value, json};
use slice_campaign_identity::{
    CampaignIdentity, ConsumptionMappingError, ExpectedAdmission, IdentityError, ProblemScopeTerms,
    ScopeTerm, episode_id, episode_within_scope, expected_admission, expected_consumer,
};

const AUTHORITY: &str =
    include_str!("../../work-engine-organization/tests/fixtures/frozen/episode-authority.json");
const OBLIGATION: &str = "rp3-initial-implementation-review";
const KEY: &str = "production-qualification-20261107:1:review-005:timestamp-projection-v1";
const SCOPE: ProblemScopeTerms<'static> = ProblemScopeTerms {
    run_id: "production-qualification-20261107",
    plan_lineage: "timestamp-projection-v1",
    slice_number: 1,
};

fn identity_json() -> Value {
    serde_json::from_str::<Value>(AUTHORITY).unwrap()["identity"].clone()
}

fn episode(edit: impl FnOnce(&mut Value)) -> Identity {
    let mut value = identity_json();
    edit(&mut value);
    Identity::parse(&parse_json(&value.to_string()).unwrap()).unwrap()
}

fn campaign() -> CampaignIdentity {
    CampaignIdentity {
        run_id: "production-qualification-20261107".into(),
        slice_number: 1,
        attempt_id: "review-005".into(),
        plan_version: "timestamp-projection-v1".into(),
    }
}

#[test]
fn the_minted_episode_id_is_the_frozen_review_episode_id() {
    assert_eq!(campaign().key(), KEY);
    assert_eq!(
        episode_id(&campaign(), OBLIGATION).unwrap(),
        "ad20341a4937167c8442fbaa9e81288d"
    );
    assert_eq!(
        identity_json()["reviewEpisodeId"],
        json!("ad20341a4937167c8442fbaa9e81288d")
    );
    assert_ne!(
        episode_id(&campaign(), "another-obligation").unwrap(),
        "ad20341a4937167c8442fbaa9e81288d"
    );
}

#[test]
fn campaign_identity_validation_keeps_the_campaign_contract_messages() {
    let refuse = |edit: fn(&mut CampaignIdentity)| {
        let mut identity = campaign();
        edit(&mut identity);
        identity.validate().unwrap_err()
    };
    assert_eq!(campaign().validate(), Ok(()));
    assert_eq!(
        refuse(|i| i.run_id = " ".into()),
        IdentityError("runId must be nonempty".into())
    );
    assert_eq!(
        refuse(|i| i.attempt_id = "a:b".into()),
        IdentityError("attemptId contains identity separator".into())
    );
    assert_eq!(
        refuse(|i| i.slice_number = 0),
        IdentityError("sliceNumber must be a positive JavaScript safe integer".into())
    );
}

#[test]
fn the_frozen_episode_is_within_its_problem_scope() {
    let identity = episode_within_scope(&episode(|_| {}), OBLIGATION, &SCOPE).unwrap();
    assert_eq!(identity, campaign());
}

type Edit = Box<dyn Fn(&mut Value)>;

#[test]
fn each_scope_term_refuses_on_its_own() {
    let cases: [(&str, Edit, ProblemScopeTerms<'static>, &str, ScopeTerm); 7] = [
        (
            "run",
            Box::new(|_| {}),
            ProblemScopeTerms {
                run_id: "other",
                ..SCOPE
            },
            OBLIGATION,
            ScopeTerm::RunId,
        ),
        (
            "slice",
            Box::new(|_| {}),
            ProblemScopeTerms {
                slice_number: 2,
                ..SCOPE
            },
            OBLIGATION,
            ScopeTerm::SliceNumber,
        ),
        (
            "lineage",
            Box::new(|_| {}),
            ProblemScopeTerms {
                plan_lineage: "other",
                ..SCOPE
            },
            OBLIGATION,
            ScopeTerm::PlanLineage,
        ),
        (
            "obligation",
            Box::new(|_| {}),
            SCOPE,
            "another-obligation",
            ScopeTerm::ObligationId,
        ),
        (
            "identity",
            Box::new(|v| v["attemptId"] = json!("a:b")),
            SCOPE,
            OBLIGATION,
            ScopeTerm::CampaignIdentity,
        ),
        (
            "attempt",
            Box::new(|v| v["attemptId"] = json!("review-006")),
            SCOPE,
            OBLIGATION,
            ScopeTerm::EpisodeId,
        ),
        (
            "episode id",
            Box::new(|v| v["reviewEpisodeId"] = json!("0".repeat(32))),
            SCOPE,
            OBLIGATION,
            ScopeTerm::EpisodeId,
        ),
    ];
    for (case, edit, scope, obligation, term) in cases {
        let error = episode_within_scope(&episode(|v| edit(v)), obligation, &scope).unwrap_err();
        assert_eq!(error.term, term, "case `{case}`: {error}");
    }
}

#[test]
fn the_consumption_mapping_derives_each_boundarys_consumer_for_its_class_only() {
    assert_eq!(
        expected_admission("builder_projection", "builder", &campaign()).unwrap(),
        ExpectedAdmission {
            boundary: "builder_projection",
            consumer: format!("slice-builder:{KEY}"),
        }
    );
    assert_eq!(
        expected_admission("campaign_terminalization", "campaign", &campaign()).unwrap(),
        ExpectedAdmission {
            boundary: "campaign_terminalization",
            consumer: format!("slice-campaign:{KEY}"),
        }
    );
    for (boundary, class) in [
        ("builder_projection", "campaign"),
        ("campaign_terminalization", "builder"),
    ] {
        assert!(matches!(
            expected_admission(boundary, class, &campaign()),
            Err(ConsumptionMappingError::ClassCrossesBoundary { .. })
        ));
    }
    assert!(matches!(
        expected_admission("operator_acceptance", "builder", &campaign()),
        Err(ConsumptionMappingError::UnsupportedBoundary(_))
    ));
    assert_eq!(
        expected_consumer("builder_projection", &campaign()).unwrap(),
        format!("slice-builder:{KEY}")
    );
    assert!(matches!(
        expected_consumer("operator_acceptance", &campaign()),
        Err(ConsumptionMappingError::UnsupportedBoundary(_))
    ));
}
