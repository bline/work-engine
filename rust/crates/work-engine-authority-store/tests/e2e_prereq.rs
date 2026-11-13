//! Host prerequisites for the real offline integration (step 0). Ignored: it needs the pinned
//! handoff trees and the built signer. Uses only the public Host surface.
#![cfg(feature = "fixture")]
mod common;
mod e2e_support;
use e2e_support::*;
use work_engine_authority_store::{adapter::input_digest, ident::sha256_hex};
use work_engine_manual_host::pi_docker::Host;

#[test]
#[ignore = "needs the pinned Pi handoff trees; set P1F_E2E_RUN and P1F_SIGNER"]
fn step0_host_prerequisites() {
    let run = std::path::PathBuf::from(std::env::var("P1F_E2E_RUN").unwrap());
    // 1. Pinned trees: manifest sha256 pins (asserted in pinned_tree) and re-inventory equality.
    let pi = pinned_tree("pi", PI_MANIFEST_SHA);
    let tc = pinned_tree("toolchain", TOOLCHAIN_MANIFEST_SHA);
    println!("ITEM1 pi_matches_manifest={}", tree_matches_manifest(&pi));
    println!(
        "ITEM1 toolchain_matches_manifest={}",
        tree_matches_manifest(&tc)
    );
    assert!(tree_matches_manifest(&pi) && tree_matches_manifest(&tc));
    // 2. Job for the selected obligation through the Host's check_job. `Host::admit` runs
    // check_job(fresh) via admit_inner; the signed schema needs an owner, so the unsigned
    // PROFILE schema (identical validation branch otherwise) is used here.
    let rig = rig(&run.join("rig"), false, now() + 250);
    let chain = chain(&rig, &run.join("store"));
    let job = chain.sched.job().clone();
    println!("ITEM2 obligation={}", chain.sched.obligation_id());
    println!(
        "ITEM2 job_id={} job_sha={}",
        job.job_id,
        input_digest(&job).unwrap()
    );
    let host = Host::admit(job.clone());
    println!("ITEM2 check_job_via_admit_ok={}", host.is_ok());
    if let Err(e) = &host {
        println!("ITEM2 error={e}");
    }
    assert!(host.is_ok());
    // 3. Signer, output under the signing scratch directory.
    let job_sha = sha256_hex(&serde_json::to_vec(&job).unwrap());
    let exe = exe_sha();
    let out = std::path::PathBuf::from(std::env::var("P1F_SIGN_DIR").unwrap()).join("step0");
    let pin = sign(&out, &job_sha, &exe, 120);
    let owner = session_task_admission::pi_pilot::PiOwner::open(pin).unwrap();
    println!(
        "ITEM3 owner_open=ok subscription={}",
        owner.is_subscription()
    );
    owner.admit(&job_sha, &exe).unwrap();
    println!("ITEM3 owner_admit=ok exe_sha={exe} job_sha={job_sha}");
}
