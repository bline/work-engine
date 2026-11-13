//! Ingestion of receipts written by the real Pi/Docker Host (attempts 2 and 3 of the offline run;
//! both have status `not_established`, so neither is a success receipt). Real bytes are used as
//! written. Where a success shape is needed, a copy of a real receipt is edited and re-hashed and
//! is named `derived`: it exercises the verifier on the real structure and is NOT a Host success
//! receipt. Fixture-scoped; provisional evidence only.
use serde_json::Value;
use work_engine_authority_store::{
    ident::sha256_hex,
    receipt::{ReceiptRefusal as R, binding_digest, verify},
};
use work_engine_manual_host::pi_docker::Job;

const A3: &[u8] = include_bytes!("fixtures/real-host-receipt-attempt3-acceptance-failed.json");
const A2: &[u8] = include_bytes!("fixtures/real-host-receipt-attempt2-export-refused.json");

struct Ids {
    exec: String,
    job_id: String,
    binding: String,
}
fn ids(bytes: &[u8]) -> Ids {
    let v: Value = serde_json::from_slice(bytes).unwrap();
    let job: Job = serde_json::from_value(v["job"].clone()).unwrap();
    Ids {
        exec: v["input_digest"].as_str().unwrap().to_owned(),
        job_id: v["job_id"].as_str().unwrap().to_owned(),
        binding: binding_digest(&job.image, &job.runtime.manifest.sha256),
    }
}
fn check(bytes: &[u8], id: &str, i: &Ids) -> Result<(), R> {
    verify(bytes, id, &i.exec, &i.job_id, &i.binding).map(|_| ())
}
/// Edit a copy of a real receipt and re-hash it so the edit reaches the later checks.
fn derived(real: &[u8], edit: impl FnOnce(&mut Value)) -> (String, Vec<u8>) {
    let mut v: Value = serde_json::from_slice(real).unwrap();
    edit(&mut v);
    let bytes = serde_json::to_vec(&v).unwrap();
    (sha256_hex(&bytes), bytes)
}
fn success(v: &mut Value) {
    v["status"] = "component_passed".into();
    v["failures"] = serde_json::json!([]);
    v["coding"]["completed"] = true.into();
}

#[test]
fn real_receipt_bytes_pass_identity_checks_and_are_refused_only_for_their_status() {
    for real in [A3, A2] {
        let i = ids(real);
        let v: Value = serde_json::from_slice(real).unwrap();
        assert_ne!(
            v["status"], "component_passed",
            "fixtures are real failures"
        );
        // The real receipt's job re-serializes to the Host's input digest (job/execution identity
        // checks come before the status check).
        assert_eq!(check(real, &sha256_hex(real), &i), Err(R::StatusNotPassed));
    }
    assert_ne!(ids(A3).exec, ids(A2).exec, "two distinct real jobs");
}
#[test]
fn derived_success_shape_verifies() {
    let (id, bytes) = derived(A3, success);
    check(&bytes, &id, &ids(A3)).unwrap();
}
#[test]
fn tampered_real_receipt_bytes_and_wrong_ids_are_refused() {
    let i = ids(A3);
    let (id, mut bytes) = derived(A3, success);
    bytes.push(b' ');
    assert_eq!(check(&bytes, &id, &i), Err(R::DigestMismatch));
    assert_eq!(check(A3, &"0".repeat(64), &i), Err(R::DigestMismatch));
    let (id, bytes) = derived(A3, success);
    let wrong = Ids {
        exec: "1".repeat(64),
        ..ids(A3)
    };
    assert_eq!(check(&bytes, &id, &wrong), Err(R::JobDigestMismatch));
    let wrong = Ids {
        job_id: "p1f-other".into(),
        ..ids(A3)
    };
    assert_eq!(check(&bytes, &id, &wrong), Err(R::JobIdMismatch));
    let wrong = Ids {
        binding: "2".repeat(64),
        ..ids(A3)
    };
    assert_eq!(check(&bytes, &id, &wrong), Err(R::BindingMismatch));
    // An edit inside the job changes the job's digest even when the receipt is re-hashed.
    let (id, bytes) = derived(A3, |v| {
        success(v);
        v["job"]["deadline_unix_seconds"] = 1.into();
    });
    assert_eq!(check(&bytes, &id, &i), Err(R::JobDigestMismatch));
}
#[test]
fn another_jobs_real_receipt_is_refused() {
    let (id3, b3) = derived(A3, success);
    let (i3, i2) = (ids(A3), ids(A2));
    check(&b3, &id3, &i3).unwrap();
    // This job's receipt presented for the other real job's execution is refused.
    assert_eq!(check(&b3, &id3, &i2), Err(R::JobDigestMismatch));
    // The other real job's receipt is refused for this execution under any status edit.
    let (id2, b2) = derived(A2, |v| v["status"] = "component_passed".into());
    assert!(check(&b2, &id2, &i3).is_err());
    // A real receipt of a run whose export was refused cannot be passed by editing its status.
    assert_eq!(check(&b2, &id2, &i2), Err(R::ExportNotFinalized));
    let mixed = Ids {
        exec: i3.exec.clone(),
        ..ids(A2)
    };
    assert!(check(&b2, &id2, &mixed).is_err());
}
#[test]
fn export_and_profile_evidence_in_the_real_structure_is_enforced() {
    let i = ids(A3);
    type Edit = Box<dyn Fn(&mut Value)>;
    let cases: Vec<(&str, R, Edit)> = vec![
        (
            "not finalized",
            R::ExportNotFinalized,
            Box::new(|v| v["coding"]["export"]["finalized"] = false.into()),
        ),
        (
            "export absent",
            R::ExportNotFinalized,
            Box::new(|v| {
                v["coding"]
                    .as_object_mut()
                    .unwrap()
                    .remove("export")
                    .map(drop)
                    .unwrap()
            }),
        ),
        (
            "not completed",
            R::ExportNotFinalized,
            Box::new(|v| v["coding"]["completed"] = false.into()),
        ),
        (
            "profile",
            R::ProfileMismatch,
            Box::new(|v| v["profile"] = "work-engine.pi-coding-fixture.v1".into()),
        ),
        (
            "job profile",
            R::JobDigestMismatch,
            Box::new(|v| v["job"]["coding"]["schema"] = "work-engine.pi-coding-fixture.v1".into()),
        ),
        (
            "provider",
            R::ProviderNotFixture,
            Box::new(|v| v["provider"] = "openai-codex".into()),
        ),
        (
            "file missing from collection",
            R::ExportInventoryMismatch,
            Box::new(|v| {
                let k = v["exported_files"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .next()
                    .unwrap()
                    .clone();
                v["exported_files"].as_object_mut().unwrap().remove(&k);
            }),
        ),
        (
            "collected digest differs",
            R::ExportInventoryMismatch,
            Box::new(|v| {
                let k = v["exported_files"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .next()
                    .unwrap()
                    .clone();
                v["exported_files"][&k]["sha256"] = "3".repeat(64).into();
            }),
        ),
        (
            "extra collected file",
            R::ExportInventoryMismatch,
            Box::new(|v| {
                let first = v["exported_files"]
                    .as_object()
                    .unwrap()
                    .values()
                    .next()
                    .unwrap()
                    .clone();
                v["exported_files"]["extra.json"] = first;
            }),
        ),
        (
            "finalized file not done",
            R::ExportInventoryMismatch,
            Box::new(|v| {
                let k = v["coding"]["export"]["files"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .next()
                    .unwrap()
                    .clone();
                v["coding"]["export"]["files"][&k]["done"] = false.into();
            }),
        ),
        (
            "collected digest not among artifacts",
            R::ExportInventoryMismatch,
            Box::new(|v| {
                let sha = v["exported_files"]
                    .as_object()
                    .unwrap()
                    .values()
                    .next()
                    .unwrap()["sha256"]
                    .clone();
                for a in v["artifacts"].as_array_mut().unwrap() {
                    if a["sha256"] == sha {
                        a["sha256"] = "4".repeat(64).into();
                    }
                }
            }),
        ),
    ];
    for (name, want, edit) in cases {
        let (id, bytes) = derived(A3, |v| {
            success(v);
            edit(v);
        });
        assert_eq!(check(&bytes, &id, &i), Err(want.clone()), "{name}");
    }
}
