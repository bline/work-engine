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

// ---------------------------------------------------------------------------------------------
// A receipt for an execution-identity-bound coding job cannot shed its coding evidence. The
// external expected identity (`ids(A3)`) is the SAME bound coding job in every case below; only
// the receipt bytes are edited and re-hashed. A status claim or a hash is not attestation.
// ---------------------------------------------------------------------------------------------

/// Delete a top-level receipt key (an absent key, as opposed to an explicit null).
fn remove(v: &mut Value, key: &str) {
    v.as_object_mut().unwrap().remove(key);
}

#[test]
fn a_bound_coding_job_cannot_shed_its_coding_evidence_from_the_receipt() {
    let i = ids(A3);
    type Edit = Box<dyn Fn(&mut Value)>;
    let cases: Vec<(&str, R, Edit)> = vec![
        // Coding evidence missing, null or malformed while the signed profile is still claimed.
        (
            "coding null",
            R::ExportNotFinalized,
            Box::new(|v| v["coding"] = Value::Null),
        ),
        (
            "coding omitted",
            R::ExportNotFinalized,
            Box::new(|v| remove(v, "coding")),
        ),
        (
            "coding a string",
            R::ExportNotFinalized,
            Box::new(|v| v["coding"] = "completed".into()),
        ),
        (
            "coding an array",
            R::ExportNotFinalized,
            Box::new(|v| v["coding"] = serde_json::json!([true])),
        ),
        (
            "coding an empty object",
            R::ExportNotFinalized,
            Box::new(|v| v["coding"] = serde_json::json!({})),
        ),
        (
            "coding completed omitted",
            R::ExportNotFinalized,
            Box::new(|v| {
                v["coding"].as_object_mut().unwrap().remove("completed");
            }),
        ),
        (
            "coding export null",
            R::ExportNotFinalized,
            Box::new(|v| v["coding"]["export"] = Value::Null),
        ),
        (
            "coding export finalized omitted",
            R::ExportNotFinalized,
            Box::new(|v| {
                v["coding"]["export"]
                    .as_object_mut()
                    .unwrap()
                    .remove("finalized");
            }),
        ),
        (
            "coding export files omitted",
            R::ExportInventoryMismatch,
            Box::new(|v| {
                v["coding"]["export"]
                    .as_object_mut()
                    .unwrap()
                    .remove("files");
            }),
        ),
        // Profile evidence removed together with, or instead of, the coding record.
        (
            "profile omitted",
            R::ProfileMismatch,
            Box::new(|v| remove(v, "profile")),
        ),
        (
            "profile null",
            R::ProfileMismatch,
            Box::new(|v| v["profile"] = Value::Null),
        ),
        (
            "profile and coding omitted",
            R::ProfileMismatch,
            Box::new(|v| {
                remove(v, "profile");
                remove(v, "coding");
            }),
        ),
        (
            "profile tampered and coding null",
            R::ProfileMismatch,
            Box::new(|v| {
                v["profile"] = "work-engine.pi-coding-fixture.v1".into();
                v["coding"] = Value::Null;
            }),
        ),
        (
            "profile replaced by a noncoding profile and coding omitted",
            R::ProfileMismatch,
            Box::new(|v| {
                v["profile"] = "work-engine.pi-observation-sdk.v1".into();
                remove(v, "coding");
            }),
        ),
    ];
    let mut wrong = Vec::new();
    for (name, want, edit) in cases {
        let (id, bytes) = derived(A3, |v| {
            success(v);
            edit(v);
        });
        let got = check(&bytes, &id, &i);
        if got != Err(want.clone()) {
            wrong.push(format!("{name}: expected Err({want:?}), got {got:?}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn a_real_failed_run_cannot_pass_by_status_edit_after_its_coding_evidence_is_dropped() {
    // Only the Host's own claim (`status`) and the dropped evidence are edited. Both real jobs
    // are bound to their own execution identity; the real receipts record a failed acceptance
    // (attempt 3) and a refused export (attempt 2).
    for (real, name) in [(A3, "attempt 3"), (A2, "attempt 2")] {
        let i = ids(real);
        for drop_coding in [
            (|v: &mut Value| v["coding"] = Value::Null) as fn(&mut Value),
            |v| remove(v, "coding"),
            |v| {
                remove(v, "coding");
                remove(v, "profile");
            },
        ] {
            let (id, bytes) = derived(real, |v| {
                v["status"] = "component_passed".into();
                v["failures"] = serde_json::json!([]);
                drop_coding(v);
            });
            assert!(
                matches!(
                    check(&bytes, &id, &i),
                    Err(R::ExportNotFinalized | R::ProfileMismatch)
                ),
                "{name}: {:?}",
                check(&bytes, &id, &i)
            );
        }
    }
}

#[test]
fn a_noncoding_job_is_not_given_coding_evidence_it_does_not_bind() {
    // The job carries no coding profile, so its execution identity differs from the coding job's.
    let noncoding = |v: &mut Value| {
        success(v);
        v["job"].as_object_mut().unwrap().remove("coding");
        let job: Job = serde_json::from_value(v["job"].clone()).unwrap();
        v["input_digest"] = sha256_hex(&serde_json::to_vec(&job).unwrap()).into();
        remove(v, "coding");
        remove(v, "profile");
    };
    let (id, plain) = derived(A3, noncoding);
    let i = ids(&plain);
    assert_ne!(i.exec, ids(A3).exec, "a different bound job");
    // Control: the genuine shape of a noncoding Host receipt (no `coding`, no `profile`) is
    // still accepted, and so is a noncoding profile string that is not the signed coding profile.
    check(&plain, &id, &i).unwrap();
    let (id, other) = derived(&plain, |v| {
        v["profile"] = "work-engine.pi-observation-sdk.v1".into();
        v["receiver"] = serde_json::json!({"synthetic": true});
    });
    check(&other, &id, &i).unwrap();

    // Contradictory coding evidence on the noncoding job is refused.
    type Edit = Box<dyn Fn(&mut Value)>;
    let cases: Vec<(&str, Edit)> = vec![
        (
            "a coding record",
            Box::new(|v| {
                let a3: Value = serde_json::from_slice(A3).unwrap();
                v["coding"] = a3["coding"].clone();
                v["profile"] = a3["profile"].clone();
            }),
        ),
        (
            "a coding record without a profile",
            Box::new(|v| {
                let a3: Value = serde_json::from_slice(A3).unwrap();
                v["coding"] = a3["coding"].clone();
            }),
        ),
        (
            "the signed coding profile without a coding record",
            Box::new(|v| {
                let a3: Value = serde_json::from_slice(A3).unwrap();
                v["profile"] = a3["profile"].clone();
            }),
        ),
    ];
    for (name, edit) in cases {
        let (id, bytes) = derived(&plain, |v| edit(v));
        assert_eq!(check(&bytes, &id, &i), Err(R::ProfileMismatch), "{name}");
    }
}

// ---------------------------------------------------------------------------------------------
// Exact correspondence of the finalized in-band file set and the collected non-`workspace/`
// export files. All cases are DERIVED verifier evidence: copies of the real attempt-3 receipt
// (status `not_established`) edited and re-hashed through public `verify`; none is a Host success
// receipt. Every case is bound to the same external identity `ids(A3)`.
// ---------------------------------------------------------------------------------------------

/// First finalized in-band path of the real receipt (map order is by key).
fn first_finalized(v: &Value) -> String {
    v["coding"]["export"]["files"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone()
}
fn finalized_files(v: &mut Value) -> &mut serde_json::Map<String, Value> {
    v["coding"]["export"]["files"].as_object_mut().unwrap()
}
fn collected_files(v: &mut Value) -> &mut serde_json::Map<String, Value> {
    v["exported_files"].as_object_mut().unwrap()
}
/// Move the collected entry of the first finalized path to `to`, leaving the finalized set alone.
fn move_collected(v: &mut Value, to: &str) -> (String, Value) {
    let k = first_finalized(v);
    let entry = collected_files(v).remove(&k).unwrap();
    collected_files(v).insert(to.to_owned(), entry.clone());
    (k, entry)
}

#[test]
fn finalized_in_band_exports_must_match_the_collected_in_band_files_exactly() {
    let i = ids(A3);
    type Edit = Box<dyn Fn(&mut Value)>;
    let cases: Vec<(&str, Edit)> = vec![
        // The finalized digest is not a string and the collected path is absent: `None == None`
        // must not count as agreement. An unreferenced in-band file keeps the counts equal.
        (
            "finalized digest omitted, collected path absent, unreferenced in-band file",
            Box::new(|v| {
                let (k, _) = move_collected(v, "unreferenced.json");
                finalized_files(v)
                    .get_mut(&k)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove("sha256");
            }),
        ),
        (
            "finalized digest null, collected path absent, unreferenced in-band file",
            Box::new(|v| {
                let (k, _) = move_collected(v, "unreferenced.json");
                finalized_files(v).get_mut(&k).unwrap()["sha256"] = Value::Null;
            }),
        ),
        (
            "finalized digest a number, collected path absent, unreferenced in-band file",
            Box::new(|v| {
                let (k, _) = move_collected(v, "unreferenced.json");
                finalized_files(v).get_mut(&k).unwrap()["sha256"] = 7.into();
            }),
        ),
        // A finalized `workspace/` path resolves against the collected workspace entry, while the
        // real in-band file stays collected but unreferenced.
        (
            "finalized path moved into workspace/, in-band collected file left unreferenced",
            Box::new(|v| {
                let k = first_finalized(v);
                let sub = format!("workspace/{k}");
                let f = finalized_files(v).remove(&k).unwrap();
                finalized_files(v).insert(sub.clone(), f);
                let entry = collected_files(v)[&k].clone();
                collected_files(v).insert(sub, entry);
            }),
        ),
        (
            "finalized path moved into workspace/, collected moved with it, unreferenced in-band file",
            Box::new(|v| {
                let k = first_finalized(v);
                let sub = format!("workspace/{k}");
                let f = finalized_files(v).remove(&k).unwrap();
                finalized_files(v).insert(sub.clone(), f);
                let entry = collected_files(v).remove(&k).unwrap();
                collected_files(v).insert(sub, entry.clone());
                collected_files(v).insert("unreferenced.json".to_owned(), entry);
            }),
        ),
        // Variants the inherited comparison already refused; they stay refused.
        (
            "finalized digest omitted, collected path present",
            Box::new(|v| {
                let k = first_finalized(v);
                finalized_files(v)
                    .get_mut(&k)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove("sha256");
            }),
        ),
        (
            "finalized path absent from collection, same-digest in-band file under another path",
            Box::new(|v| {
                move_collected(v, "renamed.json");
            }),
        ),
        (
            "finalized digest wrong",
            Box::new(|v| {
                let k = first_finalized(v);
                finalized_files(v).get_mut(&k).unwrap()["sha256"] = "5".repeat(64).into();
            }),
        ),
        (
            "finalized and collected digests both omitted for the same path",
            Box::new(|v| {
                let k = first_finalized(v);
                finalized_files(v)
                    .get_mut(&k)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove("sha256");
                collected_files(v)
                    .get_mut(&k)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove("sha256");
            }),
        ),
    ];
    let mut wrong = Vec::new();
    for (name, edit) in cases {
        let (id, bytes) = derived(A3, |v| {
            success(v);
            edit(v);
        });
        let got = check(&bytes, &id, &i);
        if got != Err(R::ExportInventoryMismatch) {
            wrong.push(format!(
                "{name}: expected Err(ExportInventoryMismatch), got {got:?}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

#[test]
fn collected_workspace_artifacts_remain_outside_the_finalized_inventory() {
    let i = ids(A3);
    let artifact_sha = |v: &Value| {
        // An artifact digest that is not an in-band export digest.
        let inband: Vec<&str> = v["exported_files"]
            .as_object()
            .unwrap()
            .values()
            .map(|e| e["sha256"].as_str().unwrap())
            .collect();
        v["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["sha256"].as_str().unwrap())
            .find(|s| !inband.contains(s))
            .unwrap()
            .to_owned()
    };
    // Controls: collected `workspace/` artifacts outside the finalized set are supported.
    let (id, bytes) = derived(A3, |v| {
        success(v);
        let sha = artifact_sha(v);
        collected_files(v).insert(
            "workspace/src/lib.rs".to_owned(),
            serde_json::json!({"path": "/derived/workspace/src/lib.rs", "sha256": sha}),
        );
    });
    check(&bytes, &id, &i).unwrap();
    let (id, bytes) = derived(A3, |v| {
        success(v);
        let sha = artifact_sha(v);
        for p in ["workspace/a.txt", "workspace/dir/b.txt"] {
            collected_files(v).insert(
                p.to_owned(),
                serde_json::json!({"path": format!("/derived/{p}"), "sha256": sha}),
            );
        }
    });
    check(&bytes, &id, &i).unwrap();
    // A collected workspace artifact still has to be among the receipt's artifacts.
    let (id, bytes) = derived(A3, |v| {
        success(v);
        collected_files(v).insert(
            "workspace/src/lib.rs".to_owned(),
            serde_json::json!({"path": "/derived/workspace/src/lib.rs", "sha256": "6".repeat(64)}),
        );
    });
    assert_eq!(check(&bytes, &id, &i), Err(R::ExportInventoryMismatch));
}
