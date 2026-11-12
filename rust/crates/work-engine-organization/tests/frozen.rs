//! 1A-9 frozen diagnostic. Requirements reconstructed from the frozen vertical derive a candidate
//! whose organizational fields are compared with the frozen selection, the episode authority and
//! the policy's completion consumer. Differences are reported, not gated: the test pins the
//! report so that a change in derivation shows up, and prints it with `--nocapture`.
//!
//! The reconstructed requirements (`fixtures/frozen/requirements.json`) are written by hand; see
//! `fixtures/frozen/provenance.json` for each value's source and sha256.

mod common;

use serde_json::Value;
use sha2::{Digest as _, Sha256};
use work_engine_organization::{
    EnforceStatus, OrganizationalRequirements, Profile, RelationshipKind, derive, enforceability,
    view,
};

const REQUIREMENTS: &[u8] = include_bytes!("fixtures/frozen/requirements.json");
const SELECTION: &[u8] = include_bytes!("fixtures/frozen/selection.json");
const EPISODE_AUTHORITY: &[u8] = include_bytes!("fixtures/frozen/episode-authority.json");
const POLICY_EXCERPT: &[u8] = include_bytes!("fixtures/frozen/job-policy-excerpt.json");
const PROVENANCE: &[u8] = include_bytes!("fixtures/frozen/provenance.json");

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}

#[test]
fn fixtures_match_their_recorded_sha256() {
    let provenance = json(PROVENANCE);
    let frozen = &provenance["frozenFiles"];
    assert_eq!(
        sha256(SELECTION),
        frozen["selection.json"]["sha256"].as_str().unwrap()
    );
    assert_eq!(
        sha256(EPISODE_AUTHORITY),
        frozen["episode-authority.json"]["sha256"].as_str().unwrap()
    );
    assert_eq!(
        json(POLICY_EXCERPT)["sourceSha256"],
        frozen["job-policy.proposed.json"]["sha256"]
    );
    assert_eq!(
        sha256(REQUIREMENTS),
        provenance["requirementsJson"]["sha256"].as_str().unwrap()
    );
}

#[derive(Debug, PartialEq, Eq)]
enum Verdict {
    Match,
    Difference,
    NotEstablished,
}

#[test]
fn frozen_vertical_diagnostic_report() {
    let selection = json(SELECTION);
    let episode = json(EPISODE_AUTHORITY);
    let completion_consumer = json(POLICY_EXCERPT)["value"].as_str().unwrap().to_owned();
    let requirements = OrganizationalRequirements::from_json_bytes(REQUIREMENTS).unwrap();
    let mut result = derive(&requirements, Profile::FixedSliceReviewV1);
    assert!(result.unresolved.is_empty(), "{:?}", result.unresolved);
    let candidate = result.candidates.remove(0);
    let organization = view(&candidate).expect("a derived candidate projects");
    let report = enforceability(&candidate, Profile::FixedSliceReviewV1);
    let mut rows: Vec<(String, Verdict, String)> = Vec::new();
    let mut note = |item: &str, verdict: Verdict, detail: String| {
        rows.push((item.to_owned(), verdict, detail));
    };

    // Obligation and the claims it requires.
    let specialist = &selection["specialists"][0];
    let obligation_id = specialist["obligationId"].as_str().unwrap();
    note(
        "obligation",
        if organization.obligations.len() == 1
            && organization.obligations[0].id.as_str() == obligation_id
            && organization.obligations[0].served_by.as_deref()
                == Some(format!("vantage:{obligation_id}").as_str())
        {
            Verdict::Match
        } else {
            Verdict::Difference
        },
        format!("frozen obligationId {obligation_id}; one reviewer vantage serves it"),
    );
    let claims = specialist["requiredClaims"].as_array().unwrap();
    note(
        "claim count vs condition count",
        if claims.len() == candidate.conditions().len() {
            Verdict::Match
        } else {
            Verdict::Difference
        },
        format!(
            "{} frozen claims, {} conditions",
            claims.len(),
            candidate.conditions().len()
        ),
    );

    // Per claim: consumer class, boundary, covered state, proposition, route, profile fields.
    for claim in claims {
        let boundary = claim["consumptionBoundary"].as_str().unwrap();
        let condition = candidate
            .conditions()
            .iter()
            .find(|c| c.boundary.as_str() == boundary)
            .unwrap_or_else(|| panic!("no condition for boundary {boundary}"));
        let consumer = claim["consumer"].as_str().unwrap();
        let expected_consumer = if consumer.starts_with("slice-builder:") {
            "vantage-builder"
        } else if consumer.starts_with("slice-campaign:") {
            "component:campaign"
        } else {
            "unrecognized"
        };
        let consumes = candidate.relationships().iter().any(|r| {
            r.kind == RelationshipKind::ConsumesAt
                && r.condition.as_ref() == Some(&condition.id)
                && r.from == expected_consumer
        });
        note(
            &format!("consumer ({boundary})"),
            if consumes {
                Verdict::Match
            } else {
                Verdict::Difference
            },
            format!("frozen consumer class prefix maps to {expected_consumer}"),
        );
        note(
            &format!("consumer identity ({boundary})"),
            Verdict::Difference,
            format!(
                "frozen consumer {consumer:?} carries run, slice and attempt identity; the candidate holds only the class"
            ),
        );
        let same = condition.covered_state.as_str() == claim["coveredState"].as_str().unwrap()
            && condition.proposition == claim["proposition"].as_str().unwrap()
            && condition.unestablished_route.as_str()
                == claim["acceptance"]["unestablishedRoute"].as_str().unwrap();
        note(
            &format!("coveredState, proposition, route ({boundary})"),
            if same {
                Verdict::Match
            } else {
                Verdict::Difference
            },
            String::new(),
        );
        let profile = &claim["profile"];
        let normative = &condition.normative;
        let capabilities_empty = profile["requiredCapabilities"]
            .as_array()
            .unwrap()
            .is_empty();
        let same = normative.continuity.as_str() == profile["continuity"].as_str().unwrap()
            && normative.capabilities.is_empty() == capabilities_empty
            && profile["integrityRequired"].as_bool() == Some(true);
        note(
            &format!("continuity, capabilities, integrity ({boundary})"),
            if same {
                Verdict::Match
            } else {
                Verdict::Difference
            },
            String::new(),
        );
        // The concrete identities live in the realization layer, not in the candidate.
        let identities: Vec<&str> = [
            profile["requiredRealization"].as_str().unwrap(),
            profile["allowedMechanisms"][0].as_str().unwrap(),
            profile["admissibleObservers"][0].as_str().unwrap(),
        ]
        .to_vec();
        let carried = identities.iter().any(|identity| {
            String::from_utf8_lossy(candidate.canonical_bytes()).contains(identity)
        });
        note(
            &format!("realization identities ({boundary})"),
            if carried {
                Verdict::Match
            } else {
                Verdict::Difference
            },
            format!(
                "{identities:?} are realization-layer values; the candidate carries none (by design)"
            ),
        );
    }

    // selection.owner is the organizational authority's, expressed by the admission act.
    note(
        "selection.owner",
        Verdict::Difference,
        format!(
            "frozen owner {:?}; the candidate carries no owner, selection is the admission act (r3 section 5)",
            selection["owner"].as_str().unwrap()
        ),
    );

    // Episode writer and readers.
    let writer = episode["writer"]["actorId"].as_str().unwrap();
    let readers: Vec<&str> = episode["readers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_str().unwrap())
        .collect();
    let access = &organization.episode_access;
    let one_host = access.len() == 1
        && access[0].writer == "component:episode_host"
        && access[0].readers == ["component:episode_host"];
    note(
        "episode access shape",
        if one_host && readers == [writer] {
            Verdict::Match
        } else {
            Verdict::Difference
        },
        format!(
            "frozen writer {writer:?} and readers {readers:?} are one actor; the candidate has one episode host as writer and sole reader"
        ),
    );
    note(
        "episode actor identity",
        Verdict::NotEstablished,
        format!(
            "frozen actor {writer:?} versus component:episode_host: the component-to-principal binding is a componentBindingRequirement that nothing in 1A establishes"
        ),
    );

    // Completion consumer and terminalization.
    let terminalizer = organization
        .terminalization
        .as_ref()
        .map(|t| t.component.as_str());
    note(
        "completionConsumer vs terminalizes",
        if terminalizer == Some("component:campaign") && completion_consumer == "campaign" {
            Verdict::Match
        } else {
            Verdict::Difference
        },
        format!(
            "frozen completionConsumer {completion_consumer:?}; terminalizes names {terminalizer:?}"
        ),
    );

    // Separation and executability: the frozen vertical's candidate is not executable.
    let acceptor = &candidate.conditions()[0].acceptor;
    note(
        "separation",
        Verdict::NotEstablished,
        format!(
            "acceptor {acceptor} is not the declared identifier of a dependent vantage; separation requirements: {}; the enforceability row is none (OC2)",
            candidate.separation_requirements().len()
        ),
    );
    let blocking: Vec<&str> = report
        .rows
        .iter()
        .filter(|row| row.status == EnforceStatus::None)
        .map(|row| row.property.as_str())
        .collect();
    note(
        "executable",
        Verdict::Difference,
        format!(
            "report.executable() = {}; none rows: {blocking:?}",
            report.executable()
        ),
    );

    for (item, verdict, detail) in &rows {
        println!("1A-9 {item}: {verdict:?} {detail}");
    }

    // Pin the report. Differences are reported, not gated: the pinned set records what is known
    // today, so a change in derivation is visible, and it does not decide acceptance.
    let pinned: Vec<(&str, &Verdict)> = rows.iter().map(|(i, v, _)| (i.as_str(), v)).collect();
    let matches = pinned.iter().filter(|(_, v)| **v == Verdict::Match).count();
    let not_established = pinned
        .iter()
        .filter(|(_, v)| **v == Verdict::NotEstablished)
        .count();
    assert_eq!(rows.len(), 18);
    assert_eq!(matches, 10);
    assert_eq!(not_established, 2);
    assert_eq!(blocking, ["separation", "separation"]);
    assert!(!report.executable());
    assert_eq!(candidate.separation_requirements().len(), 2);
}

/// The IG1A reviewer's probe (review/probe/src/main.rs) as regression tests: N1 on the frozen
/// requirements, N2 on the same input with identities in `allowed[]`.
#[test]
fn n1_probe_the_frozen_rows_have_the_revision_4_evidence_classes() {
    use work_engine_organization::EvidenceClass::{Constraint, ConstraintAndRecording};
    let requirements = OrganizationalRequirements::from_json_bytes(REQUIREMENTS).unwrap();
    let result = derive(&requirements, Profile::FixedSliceReviewV1);
    let report = enforceability(&result.candidates[0], Profile::FixedSliceReviewV1);
    let mut seen = (0, 0);
    for row in &report.rows {
        if row.property == "resultContract" {
            assert_eq!(row.evidence_class, Constraint, "{}", row.scope);
            seen.0 += 1;
        }
        if row.property == "realization:execution_confinement" {
            assert_eq!(row.evidence_class, ConstraintAndRecording, "{}", row.scope);
            seen.1 += 1;
        }
    }
    assert!(seen.0 > 0 && seen.1 > 0, "rows found: {seen:?}");
}

#[test]
fn n2_probe_identities_in_allowed_are_refused_on_the_frozen_requirements() {
    use work_engine_organization::ValidationCode;
    for identity in [
        "claude-opus-5-5",
        "2.1.322",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ] {
        let mut input: Value = json(REQUIREMENTS);
        for condition in input["acceptanceConditions"].as_array_mut().unwrap() {
            condition["realizationConstraints"] =
                serde_json::json!([{"dimension": "provider_family", "allowed": [identity]}]);
        }
        let error =
            OrganizationalRequirements::from_json_bytes(&serde_json::to_vec(&input).unwrap())
                .expect_err("the probe input must now be refused");
        assert_eq!(
            error.code,
            ValidationCode::UnrecognizedClass {
                field: "provider_family",
                value: identity.to_owned()
            }
        );
    }
}
