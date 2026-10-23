//! E1-AC3 and E1-AC4: stored pointers are resolved from the stored path and hashed (EXT-004);
//! the packet and the projection are class-checked before any field is trusted (EXT-002); each
//! basis entry resolves in the projection's `selection_metadata[]` with the right profile.
//!
//! Every test writes real packet and projection files, then the set that points at them.

mod support;
use proposal_evaluation::{Failure, check_estimate_set_bytes, sha256_hex};
use serde_json::{Value, json};
use std::path::PathBuf;
use support::*;

fn packet() -> Value {
    json!({"schema_version": 1, "proposal_id": "alpha.beta", "title": "synthetic"})
}

fn element(n: u8, profile: &str) -> Value {
    json!({
        "revision_id": format!("rev-{n}"),
        "revision_sha256": format!("{n:x}").repeat(64),
        "selection_reason": "synthetic",
        "claim": {
            "id": format!("claim-{n}"),
            "profile": profile,
            "subject": "s",
            "statement_identity": "i"
        },
        "authority_ref": "a",
        "authority_reference": {},
        "current_reliances": []
    })
}

fn projection() -> Value {
    json!({
        "schema_version": 1,
        "context_kind": "rust_reviewer_exact_claim_projection_v1",
        "selection_metadata": [
            element(1, "proposal-research-v1"),
            element(2, "proposal-research-v1"),
            element(3, "proposal-research-v1")
        ]
    })
}

type Mutation = fn(&mut Value);

struct Fixture {
    packet_path: PathBuf,
    projection_path: PathBuf,
    set: Value,
}

/// Write `packet` and `projection`, then the set pointing at them with their real digests.
fn build(name: &str, packet: &Value, projection: &Value) -> Fixture {
    let dir = fresh_dir(name);
    let packet_path = dir.join("packet.json");
    let projection_path = dir.join("projection.json");
    let packet_sha = write_json(&packet_path, packet);
    let projection_sha = write_json(&projection_path, projection);
    let set = valid_set(
        &s(&packet_path),
        &packet_sha,
        &s(&projection_path),
        &projection_sha,
    );
    Fixture {
        packet_path,
        projection_path,
        set,
    }
}

fn found(set: &Value) -> Vec<(String, String)> {
    check_estimate_set_bytes(&serde_json::to_vec(set).unwrap())
        .expect("parses")
        .errors
        .into_iter()
        .map(|e| (e.code, e.at))
        .collect()
}

fn good(name: &str) -> Fixture {
    build(name, &packet(), &projection())
}

/// Replace the projection's first element satisfying `which` with a mutated copy, rebuild.
fn with_projection(name: &str, mutate: impl FnOnce(&mut Value)) -> Vec<(String, String)> {
    let mut projection = projection();
    mutate(&mut projection);
    found(&build(name, &packet(), &projection).set)
}

#[test]
fn bound_fixture_passes_and_carries_the_limitation_notes() {
    let f = good("ptr-pass");
    let report = check_estimate_set_bytes(&serde_json::to_vec(&f.set).unwrap()).unwrap();
    assert_eq!(report.errors, vec![]);
    assert!(report.valid);
    assert_eq!(report.kind, "estimate-set");
    assert!(
        report
            .notes
            .iter()
            .any(|n| n.contains("selected in the cited projection"))
    );
    assert!(report.notes.iter().any(|n| n.contains("not established")));
}

#[test]
fn edited_packet_bytes_are_a_digest_mismatch_at_the_packet() {
    let f = good("ptr-edit-packet");
    write(
        &f.packet_path,
        "{\"schema_version\": 1, \"proposal_id\": \"alpha.beta\", \"x\": 2}\n",
    );
    let errors = found(&f.set);
    assert!(
        has(&errors, "digest_mismatch", "proposal.packet"),
        "{errors:?}"
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
}

#[test]
fn edited_projection_bytes_are_a_digest_mismatch_at_the_projection() {
    let f = good("ptr-edit-projection");
    write(&f.projection_path, "{}\n");
    let errors = found(&f.set);
    assert!(
        has(&errors, "digest_mismatch", "claim_projection"),
        "{errors:?}"
    );
    // A projection whose bytes changed is not read: no basis rule fires on its content.
    assert_eq!(errors.len(), 1, "{errors:?}");
}

#[test]
fn unreadable_pointers_are_unresolved() {
    let f = good("ptr-unreadable");
    std::fs::remove_file(&f.packet_path).unwrap();
    std::fs::remove_file(&f.projection_path).unwrap();
    let errors = found(&f.set);
    assert!(
        has(&errors, "pointer_unresolved", "proposal.packet"),
        "{errors:?}"
    );
    assert!(
        has(&errors, "pointer_unresolved", "claim_projection"),
        "{errors:?}"
    );
}

#[test]
fn a_malformed_stored_pointer_is_never_read() {
    let mut f = good("ptr-malformed");
    f.set["proposal"]["packet"]["path"] = json!("relative/packet.json");
    f.set["claim_projection"]["sha256"] = json!("not-hex");
    let errors = found(&f.set);
    assert!(has(&errors, "path_not_normalised", "proposal.packet.path"));
    assert!(has(&errors, "sha256_pattern", "claim_projection.sha256"));
    assert!(
        !errors
            .iter()
            .any(|(c, _)| c == "pointer_unresolved" || c == "digest_mismatch"),
        "{errors:?}"
    );
}

#[test]
fn a_packet_of_the_wrong_class_is_rejected() {
    let cases: [(&str, Value); 5] = [
        (
            "version",
            json!({"schema_version": 2, "proposal_id": "alpha.beta"}),
        ),
        (
            "float-version",
            json!({"schema_version": 1.5, "proposal_id": "alpha.beta"}),
        ),
        ("no-id", json!({"schema_version": 1})),
        (
            "id-pattern",
            json!({"schema_version": 1, "proposal_id": "single"}),
        ),
        ("not-object", json!([1, 2])),
    ];
    for (label, bad) in cases {
        let f = build(&format!("ptr-packet-class-{label}"), &bad, &projection());
        let errors = found(&f.set);
        assert!(
            has(&errors, "upstream_class_mismatch", "proposal.packet"),
            "{label}: {errors:?}"
        );
        assert!(
            !errors.iter().any(|(c, _)| c == "proposal_id_mismatch"),
            "{label}: {errors:?}"
        );
    }
}

#[test]
fn a_packet_naming_another_proposal_is_a_mismatch() {
    let other = json!({"schema_version": 1, "proposal_id": "gamma.delta"});
    let f = build("ptr-packet-id", &other, &projection());
    let errors = found(&f.set);
    assert!(
        has(&errors, "proposal_id_mismatch", "proposal.proposal_id"),
        "{errors:?}"
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
}

#[test]
fn a_projection_of_the_wrong_class_is_rejected_and_its_content_not_trusted() {
    let cases: [(&str, Mutation); 4] = [
        ("context-kind", |p| {
            p["context_kind"] = json!("rust_reviewer_other_v1")
        }),
        ("version", |p| p["schema_version"] = json!(2)),
        ("selection-missing", |p| {
            p.as_object_mut().unwrap().remove("selection_metadata");
        }),
        ("selection-not-array", |p| {
            p["selection_metadata"] = json!({"a": 1})
        }),
    ];
    for (label, mutate) in cases {
        let errors = with_projection(&format!("ptr-projection-class-{label}"), mutate);
        assert!(
            has(&errors, "upstream_class_mismatch", "claim_projection"),
            "{label}: {errors:?}"
        );
        // Basis entries are not resolved against a projection of the wrong class.
        assert!(
            !errors.iter().any(|(c, _)| c.starts_with("basis_")),
            "{label}: {errors:?}"
        );
    }
}

#[test]
fn a_basis_unresolved_by_each_of_the_three_keys() {
    let cases: [(&str, Mutation); 3] = [
        ("revision-id", |p| {
            p["selection_metadata"][0]["revision_id"] = json!("rev-x")
        }),
        ("revision-sha", |p| {
            p["selection_metadata"][0]["revision_sha256"] = json!("e".repeat(64));
        }),
        ("claim-id", |p| {
            p["selection_metadata"][0]["claim"]["id"] = json!("claim-x")
        }),
    ];
    for (label, mutate) in cases {
        let errors = with_projection(&format!("ptr-basis-key-{label}"), mutate);
        assert_eq!(
            errors,
            vec![(
                "basis_unresolved".to_string(),
                "estimates.0.basis.0".to_string()
            )],
            "{label}"
        );
    }
}

#[test]
fn competing_readings_basis_is_resolved_with_its_position() {
    let errors = with_projection("ptr-competing", |p| {
        p["selection_metadata"].as_array_mut().unwrap().remove(2);
    });
    assert_eq!(
        errors,
        vec![(
            "basis_unresolved".to_string(),
            "estimates.1.competing.1.basis.0".to_string()
        )]
    );
}

#[test]
fn a_basis_claim_of_another_profile_is_rejected() {
    let errors = with_projection("ptr-profile", |p| {
        p["selection_metadata"][0]["claim"]["profile"] = json!("revision-bound-review-finding-v1");
        p["selection_metadata"][1]["claim"]
            .as_object_mut()
            .unwrap()
            .remove("profile");
    });
    assert_eq!(
        errors,
        vec![
            (
                "basis_wrong_profile".to_string(),
                "estimates.0.basis.0".to_string()
            ),
            (
                "basis_wrong_profile".to_string(),
                "estimates.1.competing.0.basis.0".to_string()
            ),
        ]
    );
}

#[test]
fn a_set_without_a_projection_is_not_resolved_against_one() {
    let mut f = good("ptr-no-projection");
    f.set["claim_projection"] = Value::Null;
    let errors = found(&f.set);
    assert!(
        has(&errors, "claim_projection_presence", "claim_projection"),
        "{errors:?}"
    );
    assert!(
        !errors.iter().any(|(c, _)| c.starts_with("basis_")),
        "{errors:?}"
    );
}

#[test]
fn pointer_and_structure_errors_are_all_reported_together() {
    let mut f = good("ptr-many");
    write(&f.packet_path, "[]\n");
    f.set["evidence_cutoff"] = json!("2026-10-23T10:00:00Z");
    f.set["authority"]["priority_authorized"] = json!(true);
    write(&f.projection_path, "{}\n");
    let errors = found(&f.set);
    assert!(
        has(&errors, "cutoff_after_issue", "evidence_cutoff"),
        "{errors:?}"
    );
    assert!(
        has(
            &errors,
            "authority_asserted",
            "authority.priority_authorized"
        ),
        "{errors:?}"
    );
    assert!(
        has(&errors, "digest_mismatch", "proposal.packet"),
        "{errors:?}"
    );
    assert!(
        has(&errors, "digest_mismatch", "claim_projection"),
        "{errors:?}"
    );
}

#[test]
fn not_json_is_a_failure_not_a_report() {
    let error = check_estimate_set_bytes(b"{not json").unwrap_err();
    assert!(matches!(error, Failure::Parse(_)), "{error:?}");
    assert_eq!(error.exit_code(), 2);
}

// ---------------------------------------------------------------------------
// A stored path must name a regular file, read with a size bound (review finding F1).
// ---------------------------------------------------------------------------

/// Run `check` on a worker thread and fail the test, rather than hang it, after 20 seconds.
fn found_within_timeout(set: &Value) -> Vec<(String, String)> {
    let bytes = serde_json::to_vec(set).unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let errors = check_estimate_set_bytes(&bytes)
            .expect("parses")
            .errors
            .into_iter()
            .map(|e| (e.code, e.at))
            .collect::<Vec<_>>();
        let _ = tx.send(errors);
    });
    rx.recv_timeout(std::time::Duration::from_secs(20))
        .expect("check did not return: a stored path was opened or read without bound")
}

fn point_packet_at(f: &mut Fixture, path: &str) {
    f.set["proposal"]["packet"]["path"] = json!(path);
}

#[cfg(unix)]
fn mkfifo(path: &std::path::Path) {
    let status = std::process::Command::new("mkfifo")
        .arg(path)
        .status()
        .expect("mkfifo runs");
    assert!(status.success());
}

#[cfg(unix)]
#[test]
fn a_fifo_is_unresolved_and_never_blocks() {
    let mut f = good("ptr-fifo");
    let fifo = f.packet_path.with_file_name("fifo");
    mkfifo(&fifo);
    point_packet_at(&mut f, &s(&fifo));
    let errors = found_within_timeout(&f.set);
    assert!(
        has(&errors, "pointer_unresolved", "proposal.packet"),
        "{errors:?}"
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
}

#[cfg(unix)]
#[test]
fn a_symlink_to_a_fifo_is_refused_like_the_fifo() {
    let mut f = good("ptr-fifo-link");
    let fifo = f.packet_path.with_file_name("fifo");
    mkfifo(&fifo);
    let link = f.packet_path.with_file_name("link-to-fifo");
    std::os::unix::fs::symlink(&fifo, &link).unwrap();
    point_packet_at(&mut f, &s(&link));
    let errors = found_within_timeout(&f.set);
    assert!(
        has(&errors, "pointer_unresolved", "proposal.packet"),
        "{errors:?}"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn a_device_and_a_directory_are_unresolved() {
    for (label, path) in [
        ("zero", "/dev/zero".to_string()),
        ("null", "/dev/null".to_string()),
    ] {
        let mut f = good(&format!("ptr-device-{label}"));
        point_packet_at(&mut f, &path);
        let errors = found_within_timeout(&f.set);
        assert!(
            has(&errors, "pointer_unresolved", "proposal.packet"),
            "{label}: {errors:?}"
        );
        assert_eq!(errors.len(), 1, "{label}: {errors:?}");
    }
    let mut f = good("ptr-directory");
    let dir = s(f.packet_path.parent().unwrap());
    point_packet_at(&mut f, &dir);
    let errors = found_within_timeout(&f.set);
    assert!(
        has(&errors, "pointer_unresolved", "proposal.packet"),
        "{errors:?}"
    );
}

#[test]
fn a_file_over_the_size_bound_is_unresolved_and_one_at_the_bound_is_read() {
    use proposal_evaluation::check::MAX_UPSTREAM_BYTES;
    let mut f = good("ptr-oversize");
    let big = f.packet_path.with_file_name("big.json");
    let file = std::fs::File::create(&big).unwrap();
    file.set_len(MAX_UPSTREAM_BYTES + 1).unwrap(); // sparse: no data is written
    point_packet_at(&mut f, &s(&big));
    let errors = found_within_timeout(&f.set);
    assert!(
        has(&errors, "pointer_unresolved", "proposal.packet"),
        "{errors:?}"
    );
    assert_eq!(errors.len(), 1, "{errors:?}");
    // At exactly the bound the file is read; its zero bytes then fail the digest comparison,
    // which only happens after a successful read.
    file.set_len(MAX_UPSTREAM_BYTES).unwrap();
    let errors = found_within_timeout(&f.set);
    assert!(
        has(&errors, "digest_mismatch", "proposal.packet"),
        "{errors:?}"
    );
}

// ---------------------------------------------------------------------------
// Upstream `schema_version` is the integer 1 (review finding M15): a float with the value 1
// is another type, not version 1. `1.5` alone cannot tell `as_i64` from `as_f64`.
// ---------------------------------------------------------------------------

#[test]
fn an_upstream_schema_version_written_as_a_float_is_rejected() {
    for literal in ["1.0", "1e0", "1.00", "10e-1"] {
        let packet_text =
            format!("{{\"schema_version\": {literal}, \"proposal_id\": \"alpha.beta\"}}\n");
        let projection_text = serde_json::to_string(&projection()).unwrap().replacen(
            "\"schema_version\":1",
            &format!("\"schema_version\":{literal}"),
            1,
        );
        assert!(projection_text.contains(literal), "{literal}");

        // Packet: only the packet carries the float.
        let dir = fresh_dir(&format!("ptr-float-version-packet-{literal}"));
        let (packet_path, projection_path) = (dir.join("packet.json"), dir.join("projection.json"));
        write(&packet_path, &packet_text);
        let projection_sha = write_json(&projection_path, &projection());
        let set = valid_set(
            &s(&packet_path),
            &sha256_hex(packet_text.as_bytes()),
            &s(&projection_path),
            &projection_sha,
        );
        let errors = found(&set);
        assert!(
            has(&errors, "upstream_class_mismatch", "proposal.packet"),
            "packet {literal}: {errors:?}"
        );

        // Projection: only the projection carries the float.
        let dir = fresh_dir(&format!("ptr-float-version-projection-{literal}"));
        let (packet_path, projection_path) = (dir.join("packet.json"), dir.join("projection.json"));
        let packet_sha = write_json(&packet_path, &packet());
        write(&projection_path, &projection_text);
        let set = valid_set(
            &s(&packet_path),
            &packet_sha,
            &s(&projection_path),
            &sha256_hex(projection_text.as_bytes()),
        );
        let errors = found(&set);
        assert!(
            has(&errors, "upstream_class_mismatch", "claim_projection"),
            "projection {literal}: {errors:?}"
        );
    }
}
