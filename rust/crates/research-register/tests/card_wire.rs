use research_register::{
    card_wire::{self, Envelope},
    parse,
    use_bundle::revision,
};
#[test]
fn exact_proposed_wire_vectors() {
    for (name, bytes, digest) in [
        (
            "empty",
            include_bytes!("fixtures/card-wire-v2/success-empty.json").as_slice(),
            "ce1e6ea8bfa3ea28bcb74303180d849359a32c9116bef5226fab7989e1660e65",
        ),
        (
            "unavailable",
            include_bytes!("fixtures/card-wire-v2/success-unavailable.json").as_slice(),
            "c6d0712fe992e5af5e0a8e61b3579aff20d3444b5569e17ecd0e2ffc056d1d4e",
        ),
        (
            "refusal",
            include_bytes!("fixtures/card-wire-v2/refused-revision-unknown.json").as_slice(),
            "0ea194467a08516c42bb0daec5efdc01d06774ef2078742865294e10ca1c379f",
        ),
    ] {
        assert_eq!(revision(bytes), digest);
        let raw = parse::document(name, bytes, &mut vec![]).unwrap();
        let mut generic = card_wire::canonical(&raw).into_bytes();
        generic.push(b'\n');
        assert_eq!(generic, bytes, "historical generic codec {name}");
        if name != "refusal" {
            assert!(
                card_wire::decode(bytes).is_err(),
                "old E shape is historical only"
            );
        }
    }
}
#[test]
fn strict_codec_shape_version_and_lossless_copies() {
    let bytes = include_bytes!("fixtures/card-wire-current/success-empty.json");
    let mut v: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    v.as_object_mut().unwrap().remove("detail");
    assert!(card_wire::decode(&serde_json::to_vec(&v).unwrap()).is_err());
    v["detail"] = serde_json::Value::Null;
    v["extra"] = true.into();
    assert!(card_wire::decode(&serde_json::to_vec(&v).unwrap()).is_err());
    v.as_object_mut().unwrap().remove("extra");
    v["codec"] = "unknown".into();
    assert_eq!(
        card_wire::decode(&serde_json::to_vec(&v).unwrap()).unwrap_err(),
        "unsupported_version"
    );
    assert!(card_wire::decode(b"{\"schema\":1,\"schema\":2}").is_err());
    let j = parse::document("legacy", br#"{"z":[-0,1E+02,1.2300],"a":"/"}"#, &mut vec![]).unwrap();
    assert_eq!(
        card_wire::canonical(&j),
        r#"{"a":"/","z":[-0,1E+02,1.2300]}"#
    );
    for code in card_wire::OUTCOMES.iter().filter(|c| **c != "card") {
        let q = if *code == "bad_request" {
            None
        } else {
            Some(card_wire::CurrentCardRequest {
                decision_id: "d".into(),
                decision_revision: "git:0000000".into(),
                request_sha256: "0".repeat(64),
            })
        };
        let env = Envelope::refusal(q, code);
        assert!(card_wire::decode(&env.bytes().unwrap()).is_ok());
    }
}

#[test]
fn corrected_current_shape_vectors() {
    for bytes in [
        include_bytes!("fixtures/card-wire-current/success-empty.json").as_slice(),
        include_bytes!("fixtures/card-wire-current/success-unavailable.json").as_slice(),
        include_bytes!("fixtures/card-wire-current/refused-revision-unknown.json").as_slice(),
    ] {
        let env = card_wire::decode(bytes).unwrap();
        assert_eq!(env.bytes().unwrap(), bytes);
    }
}
