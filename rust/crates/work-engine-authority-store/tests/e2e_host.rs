//! One real offline run: real compiler output -> fixture-scoped scheduling -> signed Job -> real
//! Pi/Docker Host with the built-in fake provider -> bounded in-band exports -> real Host receipt
//! -> P1-F SQLite publication -> fresh store-backed readback. Ignored: needs Docker, the pinned
//! handoff trees and the built signer. Fixture-only; nothing here is production authority.
#![cfg(feature = "fixture")]
mod common;
mod e2e_support;
use e2e_support::*;
use serde_json::Value;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use work_engine_authority_store::{
    adapter::input_digest,
    fixture::{HostRun, run_once},
    ident::sha256_hex,
    receipt::{self, binding_digest},
    report::{declared_provider_families, realization_observed},
    store::{self, PublishOutcome, PublishRequest, Write},
};
use work_engine_manual_host::pi_docker::{Host, IMAGE};

struct Trace(std::fs::File);
impl Trace {
    fn line(&mut self, s: impl AsRef<str>) {
        println!("{}", s.as_ref());
        writeln!(self.0, "{}", s.as_ref()).unwrap();
    }
}

fn containers_for(job_id: &str) -> usize {
    let o = std::process::Command::new("/usr/bin/docker")
        .args([
            "ps",
            "-a",
            "-q",
            "--filter",
            &format!("label=work-engine.pi-job={job_id}"),
        ])
        .output()
        .unwrap();
    String::from_utf8_lossy(&o.stdout).lines().count()
}

#[test]
#[ignore = "real Docker run with the fake provider; needs P1F_E2E_DIR, P1F_E2E_ATTEMPT, P1F_SIGNER, P1F_SIGN_DIR"]
fn e2e_real_host_run_publish_and_readback() {
    let d = PathBuf::from(std::env::var("P1F_E2E_DIR").unwrap());
    let attempt = std::env::var("P1F_E2E_ATTEMPT").unwrap();
    let run: PathBuf = d.join("runs").join(format!("attempt-{attempt}"));
    std::fs::create_dir(&run).unwrap();
    let mut t = Trace(
        std::fs::File::create(d.join(format!("qual/e2e-trace-attempt-{attempt}.txt"))).unwrap(),
    );
    t.line(format!("attempt={attempt} started_unix={}", now()));

    // Chain: real requirements -> derive/view -> fixture install/admit/grant -> schedule.
    let deadline = now() + 285;
    let rig = rig(&run.join("rig"), true, deadline);
    let mut chain = chain(&rig, &run.join("store"));
    let job = chain.sched.job().clone();
    let job_sha = input_digest(&job).unwrap();
    let exe = exe_sha();
    t.line(format!(
        "obligation={} job_id={} job_sha={job_sha}",
        chain.sched.obligation_id(),
        job.job_id
    ));
    t.line(format!(
        "executable={} sha256={exe}",
        std::env::current_exe().unwrap().display()
    ));
    t.line(format!(
        "job.deadline_unix={deadline} runtime={} toolchain={}",
        job.runtime.root.display(),
        job.coding.as_ref().unwrap().toolchain.root.display()
    ));
    t.line(format!(
        "runtime_manifest_sha256={} image={IMAGE}",
        job.runtime.manifest.sha256
    ));
    std::fs::write(
        d.join(format!("qual/job-attempt-{attempt}.json")),
        serde_json::to_vec_pretty(&job).unwrap(),
    )
    .unwrap();

    // Sign for exactly this job and this executable (existing mechanism), outside Git.
    let sign_dir =
        PathBuf::from(std::env::var("P1F_SIGN_DIR").unwrap()).join(format!("attempt-{attempt}"));
    let pin = sign(&sign_dir, &job_sha, &exe, 270);
    let pin_file_sha = sha256_hex(&std::fs::read(sign_dir.join("descriptor-pin.json")).unwrap());
    t.line(format!(
        "signed_dir={} descriptor-pin.json sha256={pin_file_sha} pin.sha256={} pin.path={}",
        sign_dir.display(),
        pin.sha256,
        pin.path.display()
    ));
    t.line(format!(
        "profile.json sha256={}",
        sha256_hex(&std::fs::read(sign_dir.join("profile.json")).unwrap())
    ));
    let owner = session_task_admission::pi_pilot::PiOwner::open(pin).unwrap();
    t.line(format!(
        "PiOwner::open ok subscription={} profile_sha256={}",
        owner.is_subscription(),
        owner.profile_sha256()
    ));
    // Host::admit_signed_fixture runs owner.admit(job_sha, exe_sha) then check_job; no rebuild or move.
    let mut host = Host::admit_signed_fixture(job.clone(), owner).unwrap();
    t.line("Host::admit_signed_fixture ok (owner.admit(job_sha, exe_sha) + check_job)");

    let state_dir = job.state_dir.clone();
    let started = std::time::Instant::now();
    let outcome = run_once(&mut chain.store, &chain.sched, |_| match host.run() {
        Ok(v) => HostRun::Receipt(read_receipt(&state_dir, v["receipt_id"].as_str().unwrap())),
        Err(e) => HostRun::Unknown(e.to_string()),
    })
    .unwrap();
    t.line(format!(
        "host run wall_seconds={:.1}",
        started.elapsed().as_secs_f64()
    ));
    t.line(format!(
        "containers_remaining_for_job={}",
        containers_for(&job.job_id)
    ));
    let Some((receipt_id, bytes)) = outcome else {
        t.line("HOST OUTCOME UNKNOWN: execution marked unknown; no re-run, nothing published");
        panic!("host outcome unknown");
    };
    std::fs::write(
        d.join(format!("qual/real-receipt-attempt-{attempt}.json")),
        &bytes,
    )
    .unwrap();
    let rv: Value = serde_json::from_slice(&bytes).unwrap();
    t.line(format!(
        "receipt_id={receipt_id} sha256(bytes)={} status={} provider={} model={} profile={}",
        sha256_hex(&bytes),
        rv["status"],
        rv["provider"],
        rv["model"],
        rv["profile"]
    ));
    t.line(format!(
        "receipt phase={} daemon_exit={} removed={} rpc_verified={} failures={}",
        rv["phase"], rv["daemon_exit"], rv["removed"], rv["rpc_verified"], rv["failures"]
    ));
    t.line(format!(
        "receipt container_id={} session_id={}",
        rv["container_id"], rv["session_id"]
    ));
    t.line(format!(
        "receipt coding.export.finalized={} files={} bytes={} manifest_sha256={}",
        rv["coding"]["export"]["finalized"],
        rv["coding"]["export"]["files"]
            .as_object()
            .map_or(0, |m| m.len()),
        rv["coding"]["export"]["total_bytes"],
        rv["coding"]["export"]["manifest_sha256"]
    ));
    if let Some(files) = rv["exported_files"].as_object() {
        for (p, a) in files {
            let len = std::fs::metadata(a["path"].as_str().unwrap())
                .map(|m| m.len())
                .unwrap_or(0);
            t.line(format!("export {p} sha256={} bytes={len}", a["sha256"]));
        }
    }
    t.line(format!(
        "receipt input_digest == job_sha: {}",
        rv["input_digest"] == job_sha.as_str()
    ));

    // Publish the actual Host receipt bytes.
    let declared = declared_provider_families(common::REQ);
    let observed = realization_observed(&declared, &rv);
    t.line(format!("realization_observed={observed}"));
    let mut req = PublishRequest {
        operation_id: String::new(),
        admission_id: chain.adm.admission_id().to_owned(),
        grant_id: chain.grant.grant_id().to_owned(),
        obligation_id: chain.sched.obligation_id().to_owned(),
        execution_id: chain.sched.execution_id().to_owned(),
        org_revision: chain.adm.org_revision(),
        requirements_digest: chain.adm.requirements_digest().to_owned(),
        candidate_digest: chain.adm.candidate_digest().to_owned(),
        receipt_id: receipt_id.clone(),
        receipt_bytes: bytes.clone(),
        body: serde_json::to_vec(
            &serde_json::json!({"fixture_experiment": true, "realization_observed": observed}),
        )
        .unwrap(),
    };
    req.operation_id = req.derived_operation_id();
    let outcome = chain.store.fixture_publish(&req);
    t.line(format!("publish outcome={outcome:?}"));
    let Write::Done(PublishOutcome::Published {
        result_id,
        seq,
        replayed,
    }) = outcome
    else {
        panic!("publication did not commit: see trace")
    };
    // Fresh store-backed readback (new read-only connection) and independent re-verification.
    let r = store::read_result(chain.store.dir(), &result_id)
        .unwrap()
        .unwrap();
    t.line(format!("readback result_id={} seq={seq} replayed={replayed} class={} attestation={} domain={} label_prefix_ok={}", r.result_id, r.result_class, r.attestation, r.domain, r.label.starts_with("FIXTURE EXPERIMENT RESULT")));
    t.line(format!(
        "readback stored receipt bytes == Host receipt file bytes: {}",
        r.receipt_bytes == bytes
    ));
    let binding = binding_digest(IMAGE, &job.runtime.manifest.sha256);
    t.line(format!(
        "independent receipt::verify on readback bytes: {:?}",
        receipt::verify(
            &r.receipt_bytes,
            &r.receipt_id,
            chain.sched.execution_id(),
            &job.job_id,
            &binding
        )
        .is_ok()
    ));
    let body: Value = serde_json::from_slice(&r.body).unwrap();
    t.line(format!(
        "readback body.realization_observed={}",
        body["realization_observed"]
    ));
    let u: Value = serde_json::from_str(&r.unestablished_json).unwrap();
    t.line(format!("readback unestablished: none_rows={} separation={} authority={} component_binding={} state={}", u["none_rows"].as_array().unwrap().len(), u["separation_requirements"].as_array().unwrap().len(), u["authority_requirements"].as_array().unwrap().len(), u["component_binding_requirements"].as_array().unwrap().len(), u["state"]));
    t.line(format!(
        "integrity_check={}",
        store::integrity_check(chain.store.dir()).unwrap()
    ));
    t.line("DONE");
    assert_eq!(r.receipt_bytes, bytes);
}

fn read_receipt(state_dir: &Path, id: &str) -> Vec<u8> {
    let bytes = std::fs::read(state_dir.join("receipts").join(format!("{id}.json"))).unwrap();
    assert_eq!(sha256_hex(&bytes), id);
    bytes
}
