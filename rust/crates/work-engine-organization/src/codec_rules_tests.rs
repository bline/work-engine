//! Codec, scalar and identifier rules of contract section 17. Expected values here are stated
//! independently of the implementation (hand-derived from RFC 8785 / section 17, or computed with
//! Python `hashlib` for the digest framing), so these check rules, not stability.
//!
//! Every test passes unchanged under `--features serde_json/arbitrary_precision`.

use crate::json::{JsonValue, MAX_DEPTH, MAX_SAFE_INTEGER};
use crate::{CodecError, Digest, DigestKind, Id, IdError, Position};

fn canonical(text: &str) -> String {
    String::from_utf8(JsonValue::parse(text.as_bytes()).unwrap().canonical_bytes()).unwrap()
}

fn error(text: &str) -> CodecError {
    JsonValue::parse(text.as_bytes()).unwrap_err()
}

#[test]
fn keys_sort_by_utf16_code_unit_and_whitespace_is_dropped() {
    assert_eq!(
        canonical(" { \"b\" : 1 ,\n\"a\" : [ 2 , 3 ] } "),
        r#"{"a":[2,3],"b":1}"#
    );
    // U+1F600 is the pair D83D DE00 and sorts before U+FB33; in UTF-8 byte order it would sort after.
    assert_eq!(
        canonical("{\"\u{FB33}\":1,\"\u{1F600}\":2}"),
        "{\"\u{1F600}\":2,\"\u{FB33}\":1}"
    );
    // Array order is meaningful and kept.
    assert_eq!(canonical("[3,1,2]"), "[3,1,2]");
}

#[test]
fn strings_use_minimal_escaping_and_no_normalization() {
    assert_eq!(
        canonical(r#""\u0041\/\u007f\u2028""#),
        "\"A/\u{7f}\u{2028}\""
    );
    assert_eq!(
        canonical(r#""\u0000\u001f\b\t\n\f\r\"\\""#),
        r#""\u0000\u001f\b\t\n\f\r\"\\""#
    );
    // Surrogate pair escape decodes to the scalar value; literal and escaped spellings agree.
    assert_eq!(canonical(r#""\ud83d\ude00""#), "\"\u{1F600}\"");
    // No Unicode normalization: A + combining ring stays as two scalars, distinct from U+00C5.
    assert_eq!(canonical("\"A\u{30a}\""), "\"A\u{30a}\"");
    assert_ne!(canonical("\"A\u{30a}\""), canonical("\"\u{c5}\""));
    // No case folding or trimming.
    assert_eq!(canonical("\" Ab \""), "\" Ab \"");
}

#[test]
fn integers_are_accepted_only_within_the_safe_range() {
    assert_eq!(MAX_SAFE_INTEGER, (1i64 << 53) - 1);
    assert_eq!(canonical("9007199254740991"), "9007199254740991");
    assert_eq!(canonical("-9007199254740991"), "-9007199254740991");
    assert_eq!(canonical("0"), "0");
    // `-0` is a non-canonical spelling of 0: refused with a typed error, never rewritten.
    assert!(matches!(error("-0"), CodecError::NegativeZero { .. }));
    assert!(matches!(error("[1,-0]"), CodecError::NegativeZero { .. }));
    assert!(matches!(error("-00"), CodecError::Syntax { .. }));
    for text in [
        "9007199254740992",
        "-9007199254740992",
        "99999999999999999999999",
    ] {
        assert!(
            matches!(error(text), CodecError::IntegerOutOfRange { .. }),
            "{text}"
        );
    }
}

#[test]
fn floats_and_exponents_are_refused() {
    for text in [
        "1.0", "0.5", "-1.5", "1e2", "1E2", "1e+2", "1e-2", "56.0", "[1, 2.5]",
    ] {
        assert!(
            matches!(error(text), CodecError::NonIntegerNumber { .. }),
            "{text}"
        );
    }
}

#[test]
fn malformed_json_is_refused_with_a_syntax_error() {
    for text in [
        "",
        " ",
        "{",
        "[1,]",
        "{\"a\":1,}",
        "{\"a\" 1}",
        "01",
        "-",
        "+1",
        "tru",
        "nul",
        "\"abc",
        "\"a\u{1}b\"",
        "\"\\x\"",
        "\"\\u12\"",
        "{1:2}",
        "[1] 2",
        "{} {}",
        "\u{feff}{}",
    ] {
        assert!(
            matches!(error(text), CodecError::Syntax { .. }),
            "{text:?}: {:?}",
            error(text)
        );
    }
}

#[test]
fn invalid_utf8_is_refused() {
    let error = JsonValue::parse(b"{\"a\":\"\xff\"}").unwrap_err();
    assert_eq!(error, CodecError::InvalidUtf8 { valid_up_to: 6 });
}

#[test]
fn duplicate_object_keys_are_refused_including_escaped_spellings() {
    assert!(matches!(
        error(r#"{"a":1,"a":2}"#),
        CodecError::DuplicateKey { .. }
    ));
    assert!(
        matches!(error(r#"{"a":1,"\u0061":2}"#), CodecError::DuplicateKey { key, .. } if key == "a")
    );
    assert!(matches!(
        error(r#"{"x":{"k":1,"k":1}}"#),
        CodecError::DuplicateKey { .. }
    ));
    // The same key in different objects is fine.
    assert_eq!(
        canonical(r#"{"a":{"k":1},"b":{"k":2}}"#),
        r#"{"a":{"k":1},"b":{"k":2}}"#
    );
}

#[test]
fn unpaired_surrogate_escapes_are_refused() {
    for text in [
        r#""\ud83d""#,
        r#""\ude00""#,
        r#""\ud83d\u0041""#,
        r#""\ud83dx""#,
    ] {
        assert!(
            matches!(error(text), CodecError::UnpairedSurrogate { .. }),
            "{text}"
        );
    }
}

#[test]
fn nesting_is_bounded() {
    let ok = format!("{}{}", "[".repeat(MAX_DEPTH), "]".repeat(MAX_DEPTH));
    assert!(JsonValue::parse(ok.as_bytes()).is_ok());
    let deep = format!("{}{}", "[".repeat(MAX_DEPTH + 1), "]".repeat(MAX_DEPTH + 1));
    assert!(matches!(error(&deep), CodecError::TooDeep { .. }));
}

#[test]
fn explicit_null_is_refused_with_its_path() {
    assert!(JsonValue::parse_v0(b"{\"a\":[1,{\"b/c\":null}]}").is_err());
    let refused = JsonValue::parse_v0(b"{\"a\":[1,{\"b/c\":null}]}").unwrap_err();
    assert_eq!(
        refused,
        CodecError::ExplicitNull {
            at: Position::Path("/a/1/b~1c".into())
        }
    );
    assert!(JsonValue::parse_v0(b"{\"a\":[1,false]}").is_ok());
}

#[test]
fn digest_framing_matches_an_independent_computation() {
    // Expected values computed with Python hashlib over
    // b"work-engine.organization\x00<kind>\x00v0\x00" + canonical bytes.
    let cases = [
        (
            DigestKind::Requirements,
            "{}",
            "6e024d2aea16990b68f8f2b6eaddb4d3b0ae28a41c125433945c4758dc2fbdc0",
        ),
        (
            DigestKind::Requirements,
            r#"{"b":[true,"x"],"a":1}"#,
            "69bf5cdd4c32ad170778a936b8157d6f6fda150e2e8dbbef2098a661c5b91823",
        ),
        (
            DigestKind::Candidate,
            "{}",
            "5fa7c099706fc861b26d577510a18b56484ce15d26eabb00a12f640a892602f6",
        ),
        (
            DigestKind::Candidate,
            r#"{"a":1,"b":[true,"x"]}"#,
            "ba6cc8d42b9a3c0143e643ec1d76e9930dbd9f117ee16d07a4790d0eeb1f8f4c",
        ),
    ];
    for (kind, text, expected) in cases {
        let value = JsonValue::parse(text.as_bytes()).unwrap();
        assert_eq!(value.digest(kind).to_hex(), expected, "{kind:?} {text}");
    }
    // Kind separates domains: the same bytes under the two kinds differ.
    assert_ne!(
        Digest::frame(DigestKind::Requirements, b"{}"),
        Digest::frame(DigestKind::Candidate, b"{}")
    );
}

#[test]
fn key_and_set_order_of_authored_bytes_does_not_change_the_digest() {
    let one = JsonValue::parse(br#"{"x":{"p":1,"q":2},"y":[1,2]}"#).unwrap();
    let two = JsonValue::parse(br#"{"y":[1,2],"x":{"q":2,"p":1}}"#).unwrap();
    assert_eq!(
        one.digest(DigestKind::Requirements),
        two.digest(DigestKind::Requirements)
    );
    // Array order is a sequence property: reordering changes the digest.
    let three = JsonValue::parse(br#"{"x":{"p":1,"q":2},"y":[2,1]}"#).unwrap();
    assert_ne!(
        one.digest(DigestKind::Requirements),
        three.digest(DigestKind::Requirements)
    );
}

#[test]
fn unknown_codec_declarations_are_refused() {
    use crate::{CODEC_DOMAIN, CODEC_VERSION};
    assert_eq!(
        DigestKind::from_declaration(CODEC_DOMAIN, "requirements", CODEC_VERSION),
        Ok(DigestKind::Requirements)
    );
    assert_eq!(
        DigestKind::from_declaration(CODEC_DOMAIN, "candidate", CODEC_VERSION),
        Ok(DigestKind::Candidate)
    );
    for (domain, kind, version) in [
        (CODEC_DOMAIN, "acceptance", "v0"),
        (CODEC_DOMAIN, "admission", "v0"),
        (CODEC_DOMAIN, "grant", "v0"),
        (CODEC_DOMAIN, "requirements", "v1"),
        (CODEC_DOMAIN, "requirements", ""),
        ("work-engine.other", "requirements", "v0"),
        ("", "", ""),
    ] {
        assert!(matches!(
            DigestKind::from_declaration(domain, kind, version),
            Err(CodecError::UnknownCodec { .. })
        ));
    }
}

#[test]
fn identifier_syntax_follows_section_17_3() {
    for good in [
        "a",
        "0",
        "cond-builder",
        "vantage:rp3.initial_review",
        "a:b.c_d-e",
        &"a".repeat(128),
    ] {
        assert!(Id::parse(good).is_ok(), "{good}");
    }
    assert_eq!(Id::parse(""), Err(IdError::Empty));
    assert_eq!(
        Id::parse(&"a".repeat(129)),
        Err(IdError::TooLong { len: 129 })
    );
    for (bad, expected) in [
        ("A", IdError::InvalidStart { found: 'A' }),
        (".a", IdError::InvalidStart { found: '.' }),
        ("-a", IdError::InvalidStart { found: '-' }),
        (":a", IdError::InvalidStart { found: ':' }),
        ("_a", IdError::InvalidStart { found: '_' }),
        (" a", IdError::InvalidStart { found: ' ' }),
        (
            "aB",
            IdError::InvalidCharacter {
                index: 1,
                found: 'B',
            },
        ),
        (
            "a b",
            IdError::InvalidCharacter {
                index: 1,
                found: ' ',
            },
        ),
        (
            "a/b",
            IdError::InvalidCharacter {
                index: 1,
                found: '/',
            },
        ),
        (
            "a\n",
            IdError::InvalidCharacter {
                index: 1,
                found: '\n',
            },
        ),
        (
            "a\u{e9}",
            IdError::InvalidCharacter {
                index: 1,
                found: '\u{e9}',
            },
        ),
        ("\u{ff21}", IdError::InvalidStart { found: '\u{ff21}' }),
    ] {
        assert_eq!(Id::parse(bad), Err(expected), "{bad:?}");
    }
}

#[test]
fn identifiers_deserialize_validated_and_sort_by_byte() {
    let id: Id = serde_json::from_str("\"cond-1\"").unwrap();
    assert_eq!(id.as_str(), "cond-1");
    assert_eq!(serde_json::to_string(&id).unwrap(), "\"cond-1\"");
    assert!(serde_json::from_str::<Id>("\"Cond\"").is_err());
    assert!(serde_json::from_str::<Id>("\"\"").is_err());
    let mut ids: Vec<Id> = ["b", "a1", "a", "a-"]
        .iter()
        .map(|s| Id::parse(s).unwrap())
        .collect();
    ids.sort();
    let order: Vec<&str> = ids.iter().map(Id::as_str).collect();
    assert_eq!(order, ["a", "a-", "a1", "b"]);
}

// U2: conversion from serde_json::Value must not depend on Number internals.
#[test]
fn bytes_from_serde_json_values_canonicalize_identically_under_both_number_features() {
    // The codec never reads `serde_json::Number`: callers that build values with serde_json hand
    // over bytes. This test runs under both feature settings with the same expected bytes.
    let value = serde_json::json!({"b": [9007199254740991i64, -9007199254740991i64, 0], "a": "x", "c": {"d": true}});
    let bytes = serde_json::to_vec(&value).unwrap();
    assert_eq!(
        canonical(std::str::from_utf8(&bytes).unwrap()),
        r#"{"a":"x","b":[9007199254740991,-9007199254740991,0],"c":{"d":true}}"#
    );
    for float in ["1.0", "56.0", "0.5", "1e30", "1E2"] {
        let value: serde_json::Value = serde_json::from_str(float).unwrap();
        let bytes = serde_json::to_vec(&value).unwrap();
        let refused = JsonValue::parse(&bytes);
        assert!(
            matches!(refused, Err(CodecError::NonIntegerNumber { .. })),
            "{float}: {refused:?}"
        );
    }
}

#[test]
fn a_digit_run_that_does_not_parse_is_an_internal_error_not_zero() {
    use crate::error::{CodecError, Position};
    use crate::json::digit_run_value;
    // An empty run means the digits were all zeros.
    assert_eq!(digit_run_value("", 3), Ok(0));
    assert_eq!(
        digit_run_value("9007199254740991", 3),
        Ok(9_007_199_254_740_991)
    );
    for impossible in ["x", "1_0", "-1", " 1"] {
        assert!(
            matches!(
                digit_run_value(impossible, 3),
                Err(CodecError::Internal {
                    at: Position::Offset(3),
                    ..
                })
            ),
            "{impossible:?}"
        );
    }
}

#[test]
fn an_integer_outside_the_canonical_range_is_refused_when_serialized() {
    use crate::json::{MAX_SAFE_INTEGER, safe_integer};
    assert!(safe_integer(MAX_SAFE_INTEGER as u64).is_ok());
    assert!(safe_integer(MAX_SAFE_INTEGER as u64 + 1).is_err());
    assert!(safe_integer(u64::MAX).is_err());
}
