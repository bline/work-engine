//! RFC 8785 vectors that are not self-generated goldens: 18 upstream files (Apache-2.0, see
//! `fixtures/rfc8785/ATTRIBUTION.md`) and 4 authored vectors. Each fixture's source and sha256 are
//! in `fixtures/rfc8785/provenance.json`, and this test re-checks the sha256 of every embedded
//! fixture against it.
//!
//! Vectors whose numbers are all integers must match byte for byte. Vectors containing a
//! non-integer number (for example `56.0`, `1E30`) are outside the v0 data model and must be
//! refused with the typed error, not canonicalized.
//!
//! Embedding uses `include_bytes!`, so the crate performs no file access at test time.

use crate::CodecError;
use crate::json::JsonValue;
use sha2::{Digest as _, Sha256};

macro_rules! fixture {
    ($path:literal) => {
        include_bytes!(concat!("../tests/fixtures/rfc8785/", $path)) as &[u8]
    };
}

const PROVENANCE: &[u8] = fixture!("provenance.json");

fn hex_bytes(text: &[u8]) -> Vec<u8> {
    std::str::from_utf8(text)
        .unwrap()
        .split_whitespace()
        .map(|hex| u8::from_str_radix(hex, 16).unwrap())
        .collect()
}

/// (name, input, expected canonical bytes as published, expected as published hex dump)
type InModelVector = (&'static str, &'static [u8], &'static [u8], Vec<u8>);

fn testdata_in_model() -> Vec<InModelVector> {
    vec![
        (
            "arrays",
            fixture!("input/arrays.json"),
            fixture!("output/arrays.json"),
            hex_bytes(fixture!("outhex/arrays.txt")),
        ),
        (
            "french",
            fixture!("input/french.json"),
            fixture!("output/french.json"),
            hex_bytes(fixture!("outhex/french.txt")),
        ),
        (
            "unicode",
            fixture!("input/unicode.json"),
            fixture!("output/unicode.json"),
            hex_bytes(fixture!("outhex/unicode.txt")),
        ),
        (
            "weird",
            fixture!("input/weird.json"),
            fixture!("output/weird.json"),
            hex_bytes(fixture!("outhex/weird.txt")),
        ),
    ]
}

#[test]
fn fixtures_match_their_recorded_sha256() {
    let provenance = JsonValue::parse(PROVENANCE).unwrap();
    let JsonValue::Object(top) = provenance else {
        panic!("provenance is an object")
    };
    let files = top.iter().find(|(key, _)| key == "files").unwrap();
    let JsonValue::Array(rows) = &files.1 else {
        panic!("files is an array")
    };
    assert_eq!(rows.len(), 22);
    let embedded: [(&str, &[u8]); 22] = [
        ("input/arrays.json", fixture!("input/arrays.json")),
        ("input/french.json", fixture!("input/french.json")),
        ("input/structures.json", fixture!("input/structures.json")),
        ("input/unicode.json", fixture!("input/unicode.json")),
        ("input/values.json", fixture!("input/values.json")),
        ("input/weird.json", fixture!("input/weird.json")),
        ("outhex/arrays.txt", fixture!("outhex/arrays.txt")),
        ("outhex/french.txt", fixture!("outhex/french.txt")),
        ("outhex/structures.txt", fixture!("outhex/structures.txt")),
        ("outhex/unicode.txt", fixture!("outhex/unicode.txt")),
        ("outhex/values.txt", fixture!("outhex/values.txt")),
        ("outhex/weird.txt", fixture!("outhex/weird.txt")),
        ("output/arrays.json", fixture!("output/arrays.json")),
        ("output/french.json", fixture!("output/french.json")),
        ("output/structures.json", fixture!("output/structures.json")),
        ("output/unicode.json", fixture!("output/unicode.json")),
        ("output/values.json", fixture!("output/values.json")),
        ("output/weird.json", fixture!("output/weird.json")),
        (
            "authored_example.expected.json",
            fixture!("authored_example.expected.json"),
        ),
        (
            "authored_example.input.json",
            fixture!("authored_example.input.json"),
        ),
        (
            "authored_sorting.expected.json",
            fixture!("authored_sorting.expected.json"),
        ),
        (
            "authored_sorting.input.json",
            fixture!("authored_sorting.input.json"),
        ),
    ];
    for (name, bytes) in embedded {
        let row = rows
            .iter()
            .find_map(|row| match row {
                JsonValue::Object(members)
                    if members
                        .iter()
                        .any(|(k, v)| k == "fixture" && *v == JsonValue::String(name.into())) =>
                {
                    Some(members)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("no provenance row for {name}"));
        let recorded = row.iter().find(|(k, _)| k == "sha256").unwrap();
        let actual: String = Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(recorded.1, JsonValue::String(actual), "{name}");
    }
}

#[test]
fn testdata_vectors_in_the_data_model_match_byte_for_byte() {
    for (name, input, expected_text, expected_hex) in testdata_in_model() {
        let canonical = JsonValue::parse(input).unwrap().canonical_bytes();
        assert_eq!(canonical, expected_hex, "{name}: bytes");
        assert_eq!(canonical, expected_text, "{name}: text");
    }
}

#[test]
fn authored_sorting_vector_orders_keys_by_utf16_code_unit() {
    // RFC 8785 section 3.2.3. The 11 keys include the empty key, a prefix pair, a Latin-1 letter,
    // five astral keys (D800 DC00, D800 DC01, D834 DD1E, D83D DE00, DBFF DFFF) and two BMP keys
    // above the surrogate range (E000, FFFD). UTF-16 order puts every astral key before E000 and
    // FFFD; UTF-8 byte order puts them after, so a byte-order sort fails this vector.
    let input = fixture!("authored_sorting.input.json");
    let expected = fixture!("authored_sorting.expected.json");
    let canonical = JsonValue::parse(input).unwrap().canonical_bytes();
    assert_eq!(canonical, expected);
    let JsonValue::Object(members) = JsonValue::parse(expected).unwrap() else {
        panic!("an object")
    };
    assert_eq!(members.len(), 11);
    let keys: Vec<&str> = members.iter().map(|(key, _)| key.as_str()).collect();
    let mut by_utf8 = keys.clone();
    by_utf8.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    assert_ne!(
        keys, by_utf8,
        "the vector must distinguish UTF-16 from UTF-8 order"
    );
    let mut by_utf16 = keys.clone();
    by_utf16.sort_by_key(|key| key.encode_utf16().collect::<Vec<u16>>());
    assert_eq!(
        keys, by_utf16,
        "the expected output is in UTF-16 code-unit order"
    );
}

#[test]
fn authored_example_vector_matches_byte_for_byte() {
    // Literals, short and \u00xx escapes, solidus and DEL unescaped, non-ASCII unescaped,
    // integers up to 2^53 - 1 and nesting, within the v0 data model (RFC 8785 section 3.2.2).
    let canonical = JsonValue::parse(fixture!("authored_example.input.json"))
        .unwrap()
        .canonical_bytes();
    assert_eq!(canonical, fixture!("authored_example.expected.json"));
}

#[test]
fn vectors_with_non_integer_numbers_are_refused() {
    for (name, input) in [
        ("structures (56.0)", fixture!("input/structures.json")),
        ("values (floats, 1E30)", fixture!("input/values.json")),
    ] {
        let error = JsonValue::parse(input).unwrap_err();
        assert!(
            matches!(error, CodecError::NonIntegerNumber { .. }),
            "{name}: {error:?}"
        );
    }
}

#[test]
fn null_is_canonicalized_but_refused_under_the_v0_rules() {
    // arrays.json contains an explicit null: the published bytes are reproduced by the codec
    // layer, and the v0 entry point refuses the same input.
    let input = fixture!("input/arrays.json");
    assert!(JsonValue::parse(input).is_ok());
    let error = JsonValue::parse_v0(input).unwrap_err();
    assert_eq!(
        error,
        CodecError::ExplicitNull {
            at: crate::Position::Path("/1/10".into())
        }
    );
}
