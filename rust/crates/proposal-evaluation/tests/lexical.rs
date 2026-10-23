//! Lexical rules for every parsed JSON input (review findings F2 and F4): an object key given
//! twice at any depth is a `schema` error, never silently resolved to its last occurrence.
//! (Numbers too large for their type are in the second half of this file.)

mod support;
use proposal_evaluation::{check_estimate_set_bytes, sha256_hex};
use serde_json::{Value, json};
use support::*;

fn pairs(bytes: &[u8]) -> Vec<(String, String)> {
    check_estimate_set_bytes(bytes)
        .expect("a report, not a failure")
        .errors
        .into_iter()
        .map(|e| (e.code, e.at))
        .collect()
}

fn pretty(set: &Value) -> String {
    serde_json::to_string_pretty(set).unwrap()
}

/// Repeat the first `"key": value` line fragment `from` as `to` + `from`: `to` is inserted
/// directly before it, so the document names that key twice.
fn repeat_before(text: &str, from: &str, to: &str) -> Vec<u8> {
    assert_eq!(text.matches(from).count(), 1, "fixture needs one {from:?}");
    text.replacen(from, &format!("{to}{from}"), 1).into_bytes()
}

// ---------------------------------------------------------------------------
// F2: duplicate keys, estimate set
// ---------------------------------------------------------------------------

#[test]
fn a_clean_set_has_no_lexical_findings() {
    assert_eq!(pairs(pretty(&dummy_set()).as_bytes()).len(), 2); // only the two unread pointers
    let findings = proposal_evaluation::scan::scan(pretty(&dummy_set()).as_bytes()).unwrap();
    assert_eq!(findings, vec![]);
}

#[test]
fn a_duplicate_key_at_the_top_level_is_a_schema_error_at_that_key() {
    let text = pretty(&dummy_set());
    // The last occurrence would be valid; the first is not an id. Both are in the document.
    let bytes = repeat_before(
        &text,
        "\"estimate_set_id\":",
        "\"estimate_set_id\": \"Bad Id\",\n  ",
    );
    assert_eq!(
        pairs(&bytes),
        vec![("schema".to_string(), "estimate_set_id".to_string())]
    );
}

#[test]
fn the_top_level_schema_key_twice_is_a_schema_error_not_a_schema_mismatch() {
    let text = pretty(&dummy_set());
    let bytes = repeat_before(&text, "\"schema\":", "\"schema\": \"other-v1\",\n  ");
    assert_eq!(
        pairs(&bytes),
        vec![("schema".to_string(), "schema".to_string())]
    );
}

#[test]
fn authority_given_true_then_false_is_rejected_in_either_order() {
    let text = pretty(&dummy_set());
    for (label, first) in [("true-first", "true"), ("false-first", "false")] {
        let from = "\"priority_authorized\":";
        let bytes = repeat_before(
            &text,
            from,
            &format!("\"priority_authorized\": {first},\n    "),
        );
        assert_eq!(
            pairs(&bytes),
            vec![(
                "schema".to_string(),
                "authority.priority_authorized".to_string()
            )],
            "{label}"
        );
    }
}

#[test]
fn duplicate_keys_are_found_at_every_depth_with_the_dotted_path() {
    let text = pretty(&dummy_set());
    let sites: [(&str, &str, &str); 7] = [
        (
            "\"identity\":",
            "\"identity\": \"x\",",
            "estimator.identity",
        ),
        (
            "\"path\": \"/x/packet.json\"",
            "\"path\": \"/y\",",
            "proposal.packet.path",
        ),
        (
            "\"unit\": \"day\"",
            "\"unit\": \"x\",",
            "estimates.0.measure.scale.unit",
        ),
        (
            "\"exponent\": 0",
            "\"exponent\": 7,",
            "estimates.0.measure.scale.exponent",
        ),
        ("\"low\": {", "\"low\": 1,", "estimates.0.interval.low"),
        ("\"reason\":", "\"reason\": \"x\",", "estimates.2.reason"),
        (
            "\"level\": \"high\"",
            "\"level\": \"low\",",
            "estimates.1.competing.1.value.level",
        ),
    ];
    for (from, inserted, at) in sites {
        let bytes = repeat_before(&text, from, &format!("{inserted}\n"));
        let found = pairs(&bytes);
        assert!(has(&found, "schema", at), "{at}: {found:?}");
        assert_eq!(found.len(), 1, "{at}: {found:?}");
    }
}

#[test]
fn every_duplicate_is_listed_not_only_the_first() {
    let text = pretty(&dummy_set());
    let once = repeat_before(&text, "\"identity\":", "\"identity\": \"x\",\n    ");
    let twice = repeat_before(
        &String::from_utf8(once).unwrap(),
        "\"issued_at\":",
        "\"issued_at\": \"x\",\n  ",
    );
    let found = pairs(&twice);
    assert!(has(&found, "schema", "estimator.identity"), "{found:?}");
    assert!(has(&found, "schema", "issued_at"), "{found:?}");
}

#[test]
fn keys_equal_after_unescaping_are_duplicates() {
    let text = pretty(&dummy_set());
    // "\u0073chema" is "schema".
    let bytes = repeat_before(&text, "\"schema\":", "\"\\u0073chema\": \"x\",\n  ");
    assert_eq!(
        pairs(&bytes),
        vec![("schema".to_string(), "schema".to_string())]
    );
}

#[test]
fn equal_keys_in_different_objects_are_not_duplicates() {
    // `low` and `high` hold `quantity` in both; the fixture is valid.
    let findings = proposal_evaluation::scan::scan(pretty(&dummy_set()).as_bytes()).unwrap();
    assert_eq!(findings, vec![]);
    let findings =
        proposal_evaluation::scan::scan(br#"{"a":{"k":1},"b":{"k":2},"c":[{"k":1},{"k":2}]}"#)
            .unwrap();
    assert_eq!(findings, vec![]);
}

#[test]
fn a_document_that_does_not_parse_is_still_a_parse_failure() {
    assert!(check_estimate_set_bytes(b"{\"a\": 1,").is_err());
    assert!(check_estimate_set_bytes(b"{\"a\": 1} x").is_err());
}

// ---------------------------------------------------------------------------
// F2: duplicate keys, packet and projection
// ---------------------------------------------------------------------------

/// Write the two upstream files from raw text, then a set bound to their real digests.
fn bound_raw(name: &str, packet_text: &str, projection_text: &str) -> Value {
    let dir = fresh_dir(name);
    let (packet_path, projection_path) = (dir.join("packet.json"), dir.join("projection.json"));
    write(&packet_path, packet_text);
    write(&projection_path, projection_text);
    valid_set(
        &s(&packet_path),
        &sha256_hex(packet_text.as_bytes()),
        &s(&projection_path),
        &sha256_hex(projection_text.as_bytes()),
    )
}

fn good_packet() -> String {
    "{\"schema_version\": 1, \"proposal_id\": \"alpha.beta\"}".to_string()
}

fn good_projection() -> String {
    let element = |n: u8| {
        json!({
            "revision_id": format!("rev-{n}"),
            "revision_sha256": format!("{n:x}").repeat(64),
            "claim": {"id": format!("claim-{n}"), "profile": "proposal-research-v1"}
        })
    };
    json!({
        "schema_version": 1,
        "context_kind": "rust_reviewer_exact_claim_projection_v1",
        "selection_metadata": [element(1), element(2), element(3)]
    })
    .to_string()
}

#[test]
fn the_unmodified_upstream_fixtures_pass() {
    let set = bound_raw("lex-up-pass", &good_packet(), &good_projection());
    assert_eq!(pairs(&serde_json::to_vec(&set).unwrap()), vec![]);
}

#[test]
fn a_packet_with_a_repeated_key_is_a_schema_error_at_the_packet() {
    let cases = [
        (
            "id",
            "{\"proposal_id\": \"gamma.delta\", \"schema_version\": 1, \"proposal_id\": \"alpha.beta\"}",
            "proposal_id",
        ),
        (
            "version",
            "{\"schema_version\": 2, \"proposal_id\": \"alpha.beta\", \"schema_version\": 1}",
            "schema_version",
        ),
        (
            "nested",
            "{\"schema_version\": 1, \"proposal_id\": \"alpha.beta\", \"x\": {\"y\": 1, \"y\": 2}}",
            "x.y",
        ),
    ];
    for (label, packet, inner) in cases {
        let set = bound_raw(
            &format!("lex-up-packet-{label}"),
            packet,
            &good_projection(),
        );
        let found = check_estimate_set_bytes(&serde_json::to_vec(&set).unwrap())
            .unwrap()
            .errors;
        assert_eq!(found.len(), 1, "{label}: {found:?}");
        assert_eq!(
            (found[0].code.as_str(), found[0].at.as_str()),
            ("schema", "proposal.packet")
        );
        assert!(
            found[0].message.contains(&format!("`{inner}`")),
            "{label}: {}",
            found[0].message
        );
    }
}

#[test]
fn a_projection_with_a_repeated_key_is_a_schema_error_and_resolves_no_basis() {
    let wrong_then_right = good_projection().replacen(
        "\"profile\":\"proposal-research-v1\"",
        "\"profile\":\"revision-bound-review-finding-v1\",\"profile\":\"proposal-research-v1\"",
        1,
    );
    assert_ne!(wrong_then_right, good_projection());
    let set = bound_raw("lex-up-projection", &good_packet(), &wrong_then_right);
    let found = check_estimate_set_bytes(&serde_json::to_vec(&set).unwrap())
        .unwrap()
        .errors;
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        (found[0].code.as_str(), found[0].at.as_str()),
        ("schema", "claim_projection")
    );
    assert!(
        found[0]
            .message
            .contains("selection_metadata.0.claim.profile"),
        "{}",
        found[0].message
    );
}

#[test]
fn the_command_line_reports_a_repeated_key_with_exit_1() {
    let dir = fresh_dir("lex-cli-dup");
    let path = dir.join("set.json");
    let text = pretty(&dummy_set());
    let bytes = repeat_before(
        &text,
        "\"estimate_set_id\":",
        "\"estimate_set_id\": \"x\",\n  ",
    );
    std::fs::write(&path, bytes).unwrap();
    let run = cli(&["check", "--kind", "estimate-set", &s(&path)]);
    assert_eq!(run.code, 1);
    assert_eq!(
        run.pairs(),
        vec![("schema".to_string(), "estimate_set_id".to_string())]
    );
}
