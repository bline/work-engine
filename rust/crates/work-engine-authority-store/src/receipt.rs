//! Host-receipt verifier. Pure: it checks bytes the store itself read. The Host's own success
//! claim (`status`) is a mechanical precondition here, never attestation.
use crate::ident::{digest_fields, sha256_hex};
use serde_json::Value;
use std::collections::BTreeMap;
use work_engine_manual_host::pi_docker::coding::SIGNED_PROFILE;
use work_engine_manual_host::pi_docker::{Job, SCHEMA};

pub const PROVIDER_CLASS: &str = "fake_fixture";
pub const REQUIRED_STATUS: &str = "component_passed";
pub const ADAPTER_VERSION: &str = "p1f.adapter.v1";

/// What the store insists the job was bound to: pinned image, runtime tree manifest and the
/// adapter version. Recomputed from the receipt's own job and compared inside the transaction.
pub fn binding_digest(image: &str, runtime_manifest_sha256: &str) -> String {
    digest_fields(
        "p1f.binding.v1",
        &[
            image.as_bytes(),
            runtime_manifest_sha256.as_bytes(),
            ADAPTER_VERSION.as_bytes(),
        ],
    )
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReceiptRefusal {
    #[error("receipt bytes do not hash to the declared receipt id")]
    DigestMismatch,
    #[error("receipt is not valid JSON")]
    NotJson,
    #[error("receipt schema is not the Pi Docker custody schema")]
    WrongSchema,
    #[error("receipt job does not parse as a Host job")]
    JobUnparsable,
    #[error("receipt job does not hash to the execution id")]
    JobDigestMismatch,
    #[error("receipt input digest is not the execution id")]
    InputDigestMismatch,
    #[error("receipt job id is not the prepared job id")]
    JobIdMismatch,
    #[error("receipt status is not `component_passed`")]
    StatusNotPassed,
    #[error("receipt provider class is not `fake_fixture`")]
    ProviderNotFixture,
    #[error("receipt carries no artifact digests")]
    NoArtifacts,
    #[error("receipt job binding differs from the admission binding")]
    BindingMismatch,
    #[error("receipt is for a profile other than the signed fixture")]
    ProfileMismatch,
    #[error("receipt does not show a completed run with finalized in-band exports")]
    ExportNotFinalized,
    #[error("receipt export inventory is not the finalized in-band file set")]
    ExportInventoryMismatch,
}

pub struct VerifiedReceipt {
    pub receipt_id: String,
    pub job_id: String,
}

/// Checks (in order): bytes hash to `receipt_id`; parse; schema; job re-hash equals
/// `execution_id`; `input_digest` equals `execution_id`; `job_id` matches; status; provider
/// class; artifact digests present; job binding equals `binding`; coding evidence when the bound
/// job requires it (see `verify_real_host_run`).
pub fn verify(
    bytes: &[u8],
    receipt_id: &str,
    execution_id: &str,
    expected_job_id: &str,
    binding: &str,
) -> Result<VerifiedReceipt, ReceiptRefusal> {
    if sha256_hex(bytes) != receipt_id {
        return Err(ReceiptRefusal::DigestMismatch);
    }
    let v: Value = serde_json::from_slice(bytes).map_err(|_| ReceiptRefusal::NotJson)?;
    if v["schema"] != SCHEMA {
        return Err(ReceiptRefusal::WrongSchema);
    }
    let job: Job =
        serde_json::from_value(v["job"].clone()).map_err(|_| ReceiptRefusal::JobUnparsable)?;
    let job_bytes = serde_json::to_vec(&job).map_err(|_| ReceiptRefusal::JobUnparsable)?;
    if sha256_hex(&job_bytes) != execution_id {
        return Err(ReceiptRefusal::JobDigestMismatch);
    }
    if v["input_digest"] != execution_id {
        return Err(ReceiptRefusal::InputDigestMismatch);
    }
    if v["job_id"] != expected_job_id || job.job_id != expected_job_id {
        return Err(ReceiptRefusal::JobIdMismatch);
    }
    if v["status"] != REQUIRED_STATUS {
        return Err(ReceiptRefusal::StatusNotPassed);
    }
    if v["provider"] != PROVIDER_CLASS {
        return Err(ReceiptRefusal::ProviderNotFixture);
    }
    if v["artifacts"].as_array().is_none_or(|a| {
        a.is_empty()
            || a.iter()
                .any(|x| x["sha256"].as_str().is_none_or(|s| s.len() != 64))
    }) {
        return Err(ReceiptRefusal::NoArtifacts);
    }
    if binding_digest(&job.image, &job.runtime.manifest.sha256) != binding {
        return Err(ReceiptRefusal::BindingMismatch);
    }
    // The execution-identity-bound job decides whether a coding run is required, so the receipt
    // cannot opt out by deleting or nulling its `coding` record or `profile`. A receipt that
    // carries a coding record, or claims the signed profile, is checked the same way even when
    // the job binds no coding profile (it then contradicts its own job). Receipts of jobs
    // that bind no coding profile and claim none keep their existing semantics.
    if job.coding.is_some() || !v["coding"].is_null() || v["profile"] == SIGNED_PROFILE {
        verify_real_host_run(&v, &job)?;
    }
    Ok(VerifiedReceipt {
        receipt_id: receipt_id.to_owned(),
        job_id: expected_job_id.to_owned(),
    })
}

/// A receipt that must carry the Host's `coding` record (its bound job has a coding profile, or it
/// carries the record or claims the signed profile; every real coding Host receipt carries it) must
/// be a signed-fixture run that completed and whose in-band exports were finalized, and its collected
/// export files (outside the Host's `workspace/` namespace) must be exactly the finalized set with
/// the same digests, each also listed among the receipt's artifacts. Provisional evidence only.
fn verify_real_host_run(v: &Value, job: &Job) -> Result<(), ReceiptRefusal> {
    if v["profile"] != SIGNED_PROFILE
        || job
            .coding
            .as_ref()
            .is_none_or(|c| c.schema != SIGNED_PROFILE)
    {
        return Err(ReceiptRefusal::ProfileMismatch);
    }
    let c = &v["coding"];
    let export = &c["export"];
    if c["completed"] != true || export["finalized"] != true {
        return Err(ReceiptRefusal::ExportNotFinalized);
    }
    let (Some(finalized), Some(collected), Some(artifacts)) = (
        export["files"].as_object(),
        v["exported_files"].as_object(),
        v["artifacts"].as_array(),
    ) else {
        return Err(ReceiptRefusal::ExportInventoryMismatch);
    };
    let in_band: BTreeMap<&str, &Value> = collected
        .iter()
        .filter(|(k, _)| !k.starts_with("workspace/"))
        .map(|(k, e)| (k.as_str(), e))
        .collect();
    let artifact_sha = |sha: &str| artifacts.iter().any(|a| a["sha256"] == sha);
    // Equal counts plus every finalized path being a collected in-band path (with a finalized
    // digest that is a string and equals the collected one) make the two sets identical.
    if finalized.is_empty()
        || in_band.len() != finalized.len()
        || finalized.iter().any(|(path, f)| {
            f["done"] != true
                || !f["sha256"].as_str().is_some_and(|sha| {
                    in_band
                        .get(path.as_str())
                        .is_some_and(|e| e["sha256"].as_str() == Some(sha))
                })
        })
        || collected
            .values()
            .any(|e| e["sha256"].as_str().is_none_or(|s| !artifact_sha(s)))
    {
        return Err(ReceiptRefusal::ExportInventoryMismatch);
    }
    Ok(())
}
