//! View obligation -> `pi_docker::Job`. Pure mapping; it confers no authority. The packet is
//! written from the compiled view and obligation so the Job's input digest depends on them.
use crate::ident::{digest_fields, sha256_hex};
use serde_json::json;
use std::path::{Path, PathBuf};
use work_engine_manual_host::{
    Artifact,
    pi_docker::{Job, SCHEMA, Tree, coding},
};
use work_engine_organization::{Digest, OrganizationView};

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("obligation `{0}` is not in the view")]
    UnknownObligation(String),
    #[error("io: {0}")]
    Io(String),
}

/// Host-side inputs the adapter does not derive: pinned trees and artifacts, deadlines, profile.
#[derive(Clone, Debug)]
pub struct AdapterInputs {
    pub state_dir: PathBuf,
    pub packet_dir: PathBuf,
    pub authority: Artifact,
    pub docker: Artifact,
    pub runtime: Tree,
    pub workspace: Tree,
    pub deadline_unix_seconds: u64,
    pub command_timeout_seconds: u64,
    pub coding: Option<coding::Profile>,
}

pub fn packet_bytes(
    view: &OrganizationView,
    candidate_digest: &Digest,
    obligation: &str,
) -> Result<Vec<u8>, AdapterError> {
    let o = view
        .obligations
        .iter()
        .find(|o| o.id.as_str() == obligation)
        .ok_or_else(|| AdapterError::UnknownObligation(obligation.to_owned()))?;
    let v = json!({
        "schema": "p1f.obligation-packet.v1",
        "candidate_digest": candidate_digest.to_hex(),
        "obligation": o.id.as_str(),
        "kind": o.kind, "subject": o.subject,
        "scope_digest": o.scope_digest.to_hex(),
        "served_by": o.served_by,
        "problem_scope": {"run_id": view.problem_scope.run_id.as_str(),
            "plan_lineage": view.problem_scope.plan_lineage.as_str(),
            "slice_number": view.problem_scope.slice_number},
        "label": "fixture experiment input; not an organizational authorization",
    });
    serde_json::to_vec(&v).map_err(|e| AdapterError::Io(e.to_string()))
}

/// Deterministic: same view, candidate, obligation and inputs give the same Job and the same
/// `input_digest`; a different obligation or candidate gives a different one.
pub fn obligation_job(
    view: &OrganizationView,
    candidate_digest: &Digest,
    obligation: &str,
    inputs: &AdapterInputs,
) -> Result<Job, AdapterError> {
    let bytes = packet_bytes(view, candidate_digest, obligation)?;
    let sha = sha256_hex(&bytes);
    let path: &Path = &inputs.packet_dir;
    let file = path.join(format!("packet-{sha}.json"));
    match std::fs::read(&file) {
        Ok(existing) if existing == bytes => {}
        Ok(_) => return Err(AdapterError::Io("packet path holds other bytes".into())),
        Err(_) => std::fs::write(&file, &bytes).map_err(|e| AdapterError::Io(e.to_string()))?,
    }
    let id = digest_fields(
        "p1f.job-id.v1",
        &[candidate_digest.to_hex().as_bytes(), obligation.as_bytes()],
    );
    Ok(Job {
        schema: SCHEMA.into(),
        job_id: format!("p1f-{}", &id[..24]),
        state_dir: inputs.state_dir.clone(),
        authority: inputs.authority.clone(),
        packet: Artifact {
            path: file,
            sha256: sha,
        },
        docker: inputs.docker.clone(),
        image: work_engine_manual_host::pi_docker::IMAGE.into(),
        runtime: inputs.runtime.clone(),
        workspace: inputs.workspace.clone(),
        deadline_unix_seconds: inputs.deadline_unix_seconds,
        command_timeout_seconds: inputs.command_timeout_seconds,
        idle_seconds: 0,
        receiver: None,
        coding: inputs.coding.clone(),
    })
}

/// The execution identity the Host assigns: sha256 of the serialized Job.
pub fn input_digest(job: &Job) -> Result<String, AdapterError> {
    serde_json::to_vec(job)
        .map(|b| sha256_hex(&b))
        .map_err(|e| AdapterError::Io(e.to_string()))
}
