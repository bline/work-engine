use std::fs;
use std::path::Path;

use serde_json::Value;
use slice_campaign::codec::{
    campaign_digest, campaign_update_revision_digest, canonical_campaign,
    canonical_historical_artifact, canonical_historical_raw, historical_digest_raw, raw_sha256,
};

fn fixture(file: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(file),
    )
    .unwrap()
}

#[test]
fn sc0_owning_codec_raw_vectors() {
    let capture: Value = serde_json::from_slice(&fixture("codec-capture.json")).unwrap();
    for case in capture["cases"].as_array().unwrap() {
        let input = fixture(case["input"]["file"].as_str().unwrap());
        assert_eq!(raw_sha256(&input), case["input"]["sha256"]);
        let parsed = serde_json::from_slice::<Value>(&input);
        if !case["owners"][0]["accepted"].as_bool().unwrap() {
            assert!(parsed.is_err(), "{}", case["case"]);
            assert!(
                canonical_historical_raw(&input).is_err(),
                "{}",
                case["case"]
            );
            continue;
        }
        let _value = parsed.unwrap();
        let actual = canonical_historical_raw(&input).unwrap();
        for owner in case["owners"].as_array().unwrap() {
            let expected = fixture(owner["canonical"]["file"].as_str().unwrap());
            assert_eq!(actual, expected, "{} / {}", case["case"], owner["owner"]);
            assert_eq!(
                historical_digest_raw(&input).unwrap(),
                owner["identity_digest"],
                "{}",
                case["case"]
            );
            assert_ne!(
                raw_sha256(&fixture(
                    owner["artifact_with_final_lf"]["file"].as_str().unwrap()
                )),
                owner["identity_digest"]
            );
        }
    }
}

#[test]
fn campaign_utf16_order_and_no_final_lf_are_distinct() {
    let input: Value = serde_json::from_str("{\"\":1,\"😀\":2}").unwrap();
    let campaign = canonical_campaign(&input).unwrap();
    let historical = canonical_historical_artifact(&input).unwrap();
    assert_ne!(campaign, historical);
    assert!(!campaign.ends_with(b"\n"));
    assert!(!historical.ends_with(b"\n"));
    let simple: Value = serde_json::from_str("{\"a\":1}").unwrap();
    assert_eq!(
        campaign_digest(&simple).unwrap(),
        "015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862"
    );
    assert_eq!(
        campaign_update_revision_digest(&simple).unwrap(),
        "ab6e8620559248932cb8596d40d12cfe3f1e085f89b2f5d13e5e937d6e60049e"
    );
}
