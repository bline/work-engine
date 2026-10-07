use review_episode_core::codec::{canonical_json, field, parse_json};
use review_episode_core::identity::Identity;
use review_episode_core::state::{Phase, ReviewEpisodeState};

const STATES: &str =
    include_str!("../../../../app-server/tests/fixtures/review-episode-rust/states-v1-v2.json");

#[test]
fn legacy_v1_v2_rows_import_without_rewrite() {
    let fixture = parse_json(STATES).unwrap();
    let identity = Identity::parse(field(&fixture, "identity").unwrap()).unwrap();
    let expected_key = field(&fixture, "identityKey")
        .unwrap()
        .as_text()
        .unwrap()
        .to_string_checked()
        .unwrap();
    assert_eq!(identity.key().0, expected_key);
    let states = field(&fixture, "states").unwrap();
    let history = field(&fixture, "historyRevisions")
        .unwrap()
        .as_array()
        .unwrap();
    for (index, name) in ["v1_begin", "v2_blocked", "v2_succeeded"]
        .iter()
        .enumerate()
    {
        let frozen = field(states, name).unwrap();
        let canonical = field(frozen, "canonical")
            .unwrap()
            .as_text()
            .unwrap()
            .to_string_checked()
            .unwrap();
        let parsed = ReviewEpisodeState::from_stored_json(&canonical).unwrap();
        let revision = field(frozen, "revision")
            .unwrap()
            .as_text()
            .unwrap()
            .to_string_checked()
            .unwrap();
        assert_eq!(parsed.revision().0, revision, "{name}");
        assert_eq!(&parsed.revision().value(), &history[index], "{name}");
        assert_eq!(canonical_json(parsed.value()), canonical, "{name}");
        assert_eq!(
            parsed.schema_version() as f64,
            match field(frozen, "schemaVersion").unwrap() {
                review_episode_core::codec::JsValue::Number(number) => *number,
                _ => panic!("bad fixture schema"),
            }
        );
    }
    let blocked = field(states, "v2_blocked").unwrap();
    let blocked_json = field(blocked, "canonical")
        .unwrap()
        .as_text()
        .unwrap()
        .to_string_checked()
        .unwrap();
    assert_eq!(
        ReviewEpisodeState::from_stored_json(&blocked_json)
            .unwrap()
            .phase(),
        Phase::EvidenceUnestablished
    );
}

#[test]
fn stored_integrity_refuses_noncanonical_or_changed_rows() {
    let fixture = parse_json(STATES).unwrap();
    let canonical = field(
        field(field(&fixture, "states").unwrap(), "v1_begin").unwrap(),
        "canonical",
    )
    .unwrap()
    .as_text()
    .unwrap()
    .to_string_checked()
    .unwrap();
    assert!(ReviewEpisodeState::from_stored_json(&(canonical.clone() + "\n")).is_err());
    assert!(
        ReviewEpisodeState::from_stored_json(&canonical.replacen("initial_review", "reported", 1))
            .is_err()
    );
    assert!(ReviewEpisodeState::from_stored_json(&format!(" {canonical}")).is_err());
}
