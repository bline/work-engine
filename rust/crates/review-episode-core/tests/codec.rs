use review_episode_core::codec::{CODEC_ID, JsValue, canonical_json, digest, field, parse_json};

const FIXTURE: &str =
    include_str!("../../../../app-server/tests/fixtures/review-episode-rust/codec-v1.json");

#[test]
fn frozen_javascript_bytes_and_hashes() {
    assert_eq!(CODEC_ID, "review-episode-js-json-v1");
    let fixture = parse_json(FIXTURE).expect("fixture preserves lone surrogates");
    for vector in field(&fixture, "vectors").unwrap().as_array().unwrap() {
        let id = field(vector, "id")
            .unwrap()
            .as_text()
            .unwrap()
            .to_string_checked()
            .unwrap();
        let value = field(vector, "value").unwrap();
        let expected = field(vector, "canonical")
            .unwrap()
            .as_text()
            .unwrap()
            .to_string_checked()
            .unwrap();
        let expected_hash = field(vector, "sha256")
            .unwrap()
            .as_text()
            .unwrap()
            .to_string_checked()
            .unwrap();
        assert_eq!(canonical_json(value), expected, "{id}");
        assert_eq!(digest(value), expected_hash, "{id}");
        assert!(!expected.ends_with('\n'), "{id}");
    }
}

#[test]
fn javascript_number_boundaries_and_missing_are_distinct() {
    for (source, expected) in [
        ("-0", "0"),
        ("1e-6", "0.000001"),
        ("1e-7", "1e-7"),
        ("1e20", "100000000000000000000"),
        ("1e21", "1e+21"),
        ("9007199254740991", "9007199254740991"),
    ] {
        assert_eq!(
            canonical_json(&parse_json(source).unwrap()),
            expected,
            "{source}"
        );
    }
    let missing = parse_json(r#"{"present":null}"#).unwrap();
    let explicit = parse_json(r#"{"present":null,"absent":null}"#).unwrap();
    assert_ne!(digest(&missing), digest(&explicit));
    assert!(missing.get("absent").is_none());
    assert_eq!(explicit.get("absent"), Some(&JsValue::Null));
}

#[test]
fn raw_surrogates_and_invalid_json_are_handled_without_lossy_normalization() {
    let value = parse_json(r#"{"\ud800":"\udc00","𐀀":"paired"}"#).unwrap();
    assert_eq!(
        canonical_json(&value),
        r#"{"\ud800":"\udc00","𐀀":"paired"}"#
    );
    assert!(value.get("\u{10000}").is_some());
    for invalid in [
        r#"{"a":01}"#,
        r#"{"a":"\u00xz"}"#,
        r#"{"a":"\u+123"}"#,
        "[1,]",
        "{\"a\":1} trailing",
        "[1e400]",
    ] {
        assert!(parse_json(invalid).is_err(), "{invalid}");
    }
}
