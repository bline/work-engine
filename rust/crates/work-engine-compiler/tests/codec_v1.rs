use serde_json::{Value, json};
use work_engine_compiler::{canonical_json, sha256_hex};

#[test]
fn utf16_key_order_and_array_order_are_legacy_stable() {
    let value = json!({"\u{e000}":2,"😀":1,"2":4,"10":3,"array":[3,null,1]});
    assert_eq!(
        canonical_json(&value),
        "{\"10\":3,\"2\":4,\"array\":[3,null,1],\"😀\":1,\"\":2}"
    );
}

#[test]
fn escaping_null_and_numeric_boundaries_are_exact() {
    let value = json!({"control":"quote\" slash\\ newline\n tab\t","missing":null});
    assert_eq!(
        canonical_json(&value),
        "{\"control\":\"quote\\\" slash\\\\ newline\\n tab\\t\",\"missing\":null}"
    );
    for (input, expected) in [
        ("-0.0", "0"),
        ("0.000001", "0.000001"),
        ("0.0000001", "1e-7"),
        ("1e21", "1e+21"),
        ("1e20", "100000000000000000000"),
    ] {
        let value: Value = serde_json::from_str(input).unwrap();
        assert_eq!(canonical_json(&value), expected, "{input}");
    }
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}
