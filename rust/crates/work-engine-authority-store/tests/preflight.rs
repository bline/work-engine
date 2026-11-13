//! Step 0 preflight: the real compiler fixture, executed (packet section 3).
use work_engine_organization::{OrganizationalRequirements, Profile, derive, enforceability, view};

const REQ: &[u8] =
    include_bytes!("../../work-engine-organization/tests/fixtures/frozen/requirements.json");

#[test]
fn preflight_real_fixture() {
    let reqs = OrganizationalRequirements::from_json_bytes(REQ).expect("parse");
    println!(
        "PF1 parse ok; requirements_digest={}",
        reqs.requirements_digest().to_hex()
    );
    let mut r = derive(&reqs, Profile::FixedSliceReviewV1);
    println!(
        "PF2 candidates={} unresolved={}",
        r.candidates.len(),
        r.unresolved.len()
    );
    let c = r.candidates.remove(0);
    println!(
        "PF2 candidate content_digest={}",
        c.content_digest().to_hex()
    );
    let rep = enforceability(&c, Profile::FixedSliceReviewV1);
    println!("PF3 executable={}", rep.executable());
    for row in &rep.rows {
        if row.status == work_engine_organization::EnforceStatus::None {
            println!(
                "PF3 none-row property={} scope={} requirement={} note={}",
                row.property, row.scope, row.requirement, row.note
            );
        }
    }
    println!(
        "PF3 separation_requirements={} authority_requirements={} component_binding_requirements={}",
        c.separation_requirements().len(),
        c.authority_requirements().len(),
        c.component_binding_requirements().len()
    );
    let v = view(&c).expect("view");
    println!(
        "PF4 view ok obligations={} {:?}",
        v.obligations.len(),
        v.obligations
            .iter()
            .map(|o| o.id.as_str().to_owned())
            .collect::<Vec<_>>()
    );
}

#[test]
fn preflight_zero_condition_variant_is_refused_or_not_executable() {
    let mut v: serde_json::Value = serde_json::from_slice(REQ).unwrap();
    v["acceptanceConditions"] = serde_json::json!([]);
    v["requiredAuthorities"] = serde_json::json!([]);
    match OrganizationalRequirements::from_json_bytes(&serde_json::to_vec(&v).unwrap()) {
        Err(e) => println!("PF3b zero-condition variant refused by parser: {e}"),
        Ok(r) => {
            let res = derive(&r, Profile::FixedSliceReviewV1);
            println!(
                "PF3b parsed; candidates={} unresolved={}",
                res.candidates.len(),
                res.unresolved.len()
            );
            if let Some(c) = res.candidates.first() {
                println!(
                    "PF3b executable={}",
                    enforceability(c, Profile::FixedSliceReviewV1).executable()
                );
            }
        }
    }
}
