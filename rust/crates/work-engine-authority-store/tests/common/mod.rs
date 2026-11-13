#![allow(dead_code)]
//! Shared test support. The compiler is the real one and the requirements are the real frozen
//! fixture. Synthetic Host receipts imitate the real receipt's structure for the no-Docker tier
//! only (labelled `synthetic_test_fixture`); the real Host receipt is used only in the Docker run.
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use work_engine_authority_store::{adapter::AdapterInputs, ident::sha256_hex};
use work_engine_manual_host::{
    Artifact,
    pi_docker::{Job, SCHEMA, Tree, inventory},
};
use work_engine_organization::{
    OrganizationCandidate, OrganizationView, OrganizationalRequirements, Profile, derive, view,
};

pub const REQ: &[u8] =
    include_bytes!("../../../work-engine-organization/tests/fixtures/frozen/requirements.json");

pub fn candidate() -> (
    OrganizationalRequirements,
    OrganizationCandidate,
    OrganizationView,
) {
    let reqs = OrganizationalRequirements::from_json_bytes(REQ).expect("parse");
    let mut r = derive(&reqs, Profile::FixedSliceReviewV1);
    assert_eq!(r.candidates.len(), 1);
    assert!(r.unresolved.is_empty());
    let c = r.candidates.remove(0);
    let v = view(&c).expect("view");
    (reqs, c, v)
}

/// A variant of the real requirements (different problemScope slice number) that the real
/// compiler derives into a different candidate. Used for supersession tests.
pub fn variant_candidate() -> OrganizationCandidate {
    let mut v: Value = serde_json::from_slice(REQ).unwrap();
    let n = v["problemScope"]["sliceNumber"].as_u64().unwrap();
    v["problemScope"]["sliceNumber"] = json!(n + 1);
    let reqs = OrganizationalRequirements::from_json_bytes(&serde_json::to_vec(&v).unwrap())
        .expect("variant parses");
    let mut r = derive(&reqs, Profile::FixedSliceReviewV1);
    assert_eq!(r.candidates.len(), 1);
    r.candidates.remove(0)
}

pub fn write_art(dir: &Path, name: &str, bytes: &[u8]) -> Artifact {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    Artifact {
        path,
        sha256: sha256_hex(bytes),
    }
}

pub fn tree(root: &Path, manifest_dir: &Path, name: &str) -> Tree {
    let inv = inventory(root).unwrap();
    Tree {
        root: root.to_owned(),
        manifest: write_art(manifest_dir, name, &serde_json::to_vec(&inv).unwrap()),
    }
}

pub struct Env {
    pub _tmp: tempfile::TempDir,
    pub root: PathBuf,
    pub inputs: AdapterInputs,
}

pub fn env() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let root = std::fs::canonicalize(tmp.path()).unwrap();
    for d in ["runtime", "workspace", "state", "meta", "packets"] {
        std::fs::create_dir(root.join(d)).unwrap();
    }
    std::fs::write(root.join("runtime/pi.txt"), b"synthetic runtime tree").unwrap();
    std::fs::write(
        root.join("workspace/readme.txt"),
        b"synthetic workspace tree",
    )
    .unwrap();
    let inputs = AdapterInputs {
        state_dir: root.join("state"),
        packet_dir: root.join("packets"),
        authority: write_art(
            &root.join("meta"),
            "authority.json",
            b"{\"synthetic\":\"authority\"}",
        ),
        docker: write_art(
            &root.join("meta"),
            "docker.json",
            b"{\"synthetic\":\"docker\"}",
        ),
        runtime: tree(
            &root.join("runtime"),
            &root.join("meta"),
            "runtime-manifest.json",
        ),
        workspace: tree(
            &root.join("workspace"),
            &root.join("meta"),
            "workspace-manifest.json",
        ),
        deadline_unix_seconds: 4_102_444_800,
        command_timeout_seconds: 10,
        coding: None,
    };
    Env {
        _tmp: tmp,
        root,
        inputs,
    }
}

/// Synthetic receipt imitating `pi_docker::Host::receipt`'s structure for the fields the store
/// verifies. `mutate` edits the JSON before serialization.
pub fn synthetic_receipt(job: &Job, mutate: impl FnOnce(&mut Value)) -> (String, Vec<u8>) {
    let input_digest = sha256_hex(&serde_json::to_vec(job).unwrap());
    let mut v = json!({
        "schema": SCHEMA, "job": job, "job_id": job.job_id, "input_digest": input_digest,
        "status": "component_passed", "acceptance": "owner_pending",
        "domain_effect_settlement": "not_claimed", "provider": "fake_fixture",
        "artifacts": [{"path": "/synthetic/a", "sha256": "a".repeat(64)}],
        "synthetic_test_fixture": true,
    });
    mutate(&mut v);
    let bytes = serde_json::to_vec(&v).unwrap();
    (sha256_hex(&bytes), bytes)
}

#[cfg(feature = "fixture")]
pub mod harness {
    use super::*;
    use std::time::Duration;
    use work_engine_authority_store::{
        fixture::*,
        store::{PublishRequest, Store},
    };

    pub struct H {
        pub env: Env,
        pub dir: PathBuf,
        pub store: Store,
        pub cand: OrganizationCandidate,
        pub view: OrganizationView,
        pub adm: FixtureAdmission,
        pub grant: FixtureGrant,
        pub sched: FixtureSchedule,
        pub receipt_id: String,
        pub receipt_bytes: Vec<u8>,
        pub finished: bool,
    }

    pub fn store_dir(env: &Env, name: &str) -> PathBuf {
        env.root.join(name)
    }

    /// Install, admit (fixture), grant, schedule; execution prepared and, if `finish`, finished
    /// with a synthetic receipt whose job is the scheduled job.
    pub fn harness_with(busy_ms: u64, finish: bool) -> H {
        let env = env();
        let dir = store_dir(&env, "store");
        let mut store =
            Store::create(&dir, "p1f-test-store", Duration::from_millis(busy_ms)).unwrap();
        let (_, cand, view) = candidate();
        store
            .fixture_install(&FixtureAcceptance::synthetic("test-acceptance"), &cand)
            .unwrap();
        let binding = AdmissionBinding {
            image: work_engine_manual_host::pi_docker::IMAGE.into(),
            runtime_manifest_sha256: env.inputs.runtime.manifest.sha256.clone(),
        };
        let adm = store.fixture_admit(&cand, &binding).unwrap();
        let grant = store.fixture_grant(&adm).unwrap();
        let ob = view.obligations[0].id.as_str().to_owned();
        let sched = store
            .fixture_schedule(&adm, &view, &cand, &ob, &env.inputs)
            .unwrap();
        let (receipt_id, receipt_bytes) = synthetic_receipt(sched.job(), |_| {});
        if finish {
            let rb = receipt_bytes.clone();
            run_once(&mut store, &sched, move |_| HostRun::Receipt(rb)).unwrap();
        }
        H {
            env,
            dir,
            store,
            cand,
            view,
            adm,
            grant,
            sched,
            receipt_id,
            receipt_bytes,
            finished: finish,
        }
    }
    pub fn harness() -> H {
        harness_with(2000, true)
    }

    impl H {
        pub fn request(&self) -> PublishRequest {
            PublishRequest {
                operation_id: String::new(),
                admission_id: self.adm.admission_id().to_owned(),
                grant_id: self.grant.grant_id().to_owned(),
                obligation_id: self.sched.obligation_id().to_owned(),
                execution_id: self.sched.execution_id().to_owned(),
                org_revision: self.adm.org_revision(),
                requirements_digest: self.adm.requirements_digest().to_owned(),
                candidate_digest: self.adm.candidate_digest().to_owned(),
                receipt_id: self.receipt_id.clone(),
                receipt_bytes: self.receipt_bytes.clone(),
                body: b"{\"fixture_experiment\":true}".to_vec(),
            }
            .with_derived_operation_id()
        }
        pub fn count(&self, table: &str) -> i64 {
            self.store
                .connection()
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        }
    }

    pub trait WithOp {
        fn with_derived_operation_id(self) -> Self;
    }
    impl WithOp for PublishRequest {
        fn with_derived_operation_id(mut self) -> Self {
            self.operation_id = self.derived_operation_id();
            self
        }
    }
}
