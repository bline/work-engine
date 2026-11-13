//! P1-F preflight (packet section 4), executed. Items 1-5 of the revised preflight. Item 6 (an
//! actual simulated-provider run) is outside this file.
mod common;
use common::*;
use std::io::Write;
use work_engine_authority_store::{
    adapter::{input_digest, obligation_job},
    ident::sha256_hex,
    report::{report_json, unestablished_json},
};
use work_engine_organization::{EnforceStatus, Profile, enforceability};

const PINNED_REQUIREMENTS_SHA256: &str =
    "7085318950aceb6f7071d29dec83e13da610b20d110f435e3399c9de57d3b292";
const PINNED_REQUIREMENTS_DIGEST: &str =
    "81b0e582a9376caeb6b491e2fc1730276252649ec83d1326100aec46d5b351b0";
const PINNED_CANDIDATE_DIGEST: &str =
    "e47e5dd9ddcf626bb960214278d8c5fb05fcd9f2e0522ede3dc918ceaa08fca3";

fn out(name: &str) -> Option<std::fs::File> {
    std::env::var("P1F_QUAL_DIR")
        .ok()
        .map(|d| std::fs::File::create(std::path::Path::new(&d).join(name)).unwrap())
}

#[test]
fn pf1_pf2_fixture_parses_and_derives_the_intended_candidate() {
    assert_eq!(sha256_hex(REQ), PINNED_REQUIREMENTS_SHA256);
    let (reqs, c, _) = candidate();
    assert_eq!(
        reqs.requirements_digest().to_hex(),
        PINNED_REQUIREMENTS_DIGEST
    );
    assert_eq!(c.content_digest().to_hex(), PINNED_CANDIDATE_DIGEST);
    // Determinism: a second derivation gives the same candidate bytes.
    let (_, c2, _) = candidate();
    assert_eq!(c.canonical_bytes(), c2.canonical_bytes());
    println!(
        "PF1 PF2 requirements_digest={PINNED_REQUIREMENTS_DIGEST} content_digest={PINNED_CANDIDATE_DIGEST}"
    );
}

#[test]
fn pf3_complete_enforceability_report_is_saved_and_has_the_expected_unqualified_rows() {
    let (_, c, _) = candidate();
    let report = enforceability(&c, Profile::FixedSliceReviewV1);
    assert!(!report.executable());
    let none: Vec<_> = report
        .rows
        .iter()
        .filter(|r| r.status == EnforceStatus::None)
        .collect();
    assert_eq!(
        none.len(),
        2,
        "expected exactly the two OC2 separation rows"
    );
    assert!(
        none.iter()
            .all(|r| r.property == "separation" && r.note.contains("OC2"))
    );
    assert_eq!(c.separation_requirements().len(), 2);
    assert!(!c.authority_requirements().is_empty());
    assert!(!c.component_binding_requirements().is_empty());
    let doc = serde_json::json!({
        "fixture_sha256": PINNED_REQUIREMENTS_SHA256,
        "candidate_content_digest": c.content_digest().to_hex(),
        "report": report_json(&report),
        "unestablished": unestablished_json(&c, &report),
        "row_count": report.rows.len(),
    });
    if let Some(mut f) = out("enforceability-report.json") {
        f.write_all(serde_json::to_string_pretty(&doc).unwrap().as_bytes())
            .unwrap();
    }
    println!(
        "PF3 rows={} none_rows={} authority_requirements={} component_binding_requirements={}",
        report.rows.len(),
        none.len(),
        c.authority_requirements().len(),
        c.component_binding_requirements().len()
    );
}

#[test]
fn pf2_view_is_valid_and_the_obligation_maps_deterministically_to_a_job() {
    let (_, c, v) = candidate();
    assert_eq!(v.obligations.len(), 1);
    let ob = v.obligations[0].id.as_str().to_owned();
    let e = env();
    let j1 = obligation_job(&v, &c.content_digest(), &ob, &e.inputs).unwrap();
    let j2 = obligation_job(&v, &c.content_digest(), &ob, &e.inputs).unwrap();
    let e2 = env();
    let j3 = obligation_job(&v, &c.content_digest(), &ob, &e2.inputs).unwrap();
    let (d1, d2) = (input_digest(&j1).unwrap(), input_digest(&j2).unwrap());
    assert_eq!(d1, d2, "same inputs, same input digest");
    // Same logical inputs in a different temp location differ only by paths, so the digest is
    // allowed to differ; the packet bytes (what the adapter derives) must not.
    assert_eq!(j1.packet.sha256, j3.packet.sha256);
    assert_eq!(j1.job_id, j3.job_id);
    // A changed input gives a different digest.
    let mut changed = e.inputs.clone();
    changed.command_timeout_seconds += 1;
    let j4 = obligation_job(&v, &c.content_digest(), &ob, &changed).unwrap();
    assert_ne!(d1, input_digest(&j4).unwrap());
    let other = variant_candidate();
    let other_view = work_engine_organization::view(&other).unwrap();
    let ob2 = other_view.obligations[0].id.as_str().to_owned();
    let j5 = obligation_job(&other_view, &other.content_digest(), &ob2, &e.inputs).unwrap();
    assert_ne!(
        j1.job_id, j5.job_id,
        "a different candidate changes the job"
    );
    assert!(obligation_job(&v, &c.content_digest(), "no-such-obligation", &e.inputs).is_err());
    println!(
        "PF2b obligation={ob} input_digest={d1} job_id={}",
        j1.job_id
    );
}
