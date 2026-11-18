//! The job inputs of one hosted review job and their record. The job itself (job, authority and
//! packet JSON, budget, routes) is written by the dev vertical's existing path
//! (`records/dev-vertical-h2-route-20261117/analyze.py`, which takes a snapshot directory and a
//! task file); this crate writes no runtime admission.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use work_engine_review_delivery::{FrozenDelivery, Transport};
use work_engine_review_preparation::{RealizationClaim, Subject};

use crate::revision::{self, FrozenWorld, Profile};
use crate::subject::{self, Snapshot};
use crate::task::{self, Limits};
use crate::{Refusal, secrets, sha256_hex};

/// v2: the explicitly selected provider `profile` is recorded.
pub const RECORD_SCHEMA: &str = "work-engine.review-hosted-job-inputs.v2";
pub const RECORD_STANDING: &str = "job_inputs_only_not_an_admission";

/// One blocker of the chosen world's preparation, as recorded (none is discharged).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockerRef {
    pub id: String,
    pub class: String,
}

/// The record that binds the requirements revision, preparation, contract, task, snapshot and
/// frozen-world refusal of one job. Every value is recomputed by [`crate::check`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobInputs {
    pub schema: String,
    pub standing: String,
    /// The provider profile, chosen explicitly; it determines revision, claim, model, tools and
    /// template.
    pub profile: Profile,
    pub revision: String,
    pub requirements_sha256: String,
    pub run_id: String,
    pub subject: Subject,
    pub parent: String,
    pub job_id: String,
    pub transport: Transport,
    pub requested_model: String,
    pub provider: String,
    pub realization_claim: RealizationClaim,
    pub blockers: Vec<BlockerRef>,
    pub contract_sha256: String,
    pub contract_bytes: usize,
    pub template: String,
    pub template_sha256: String,
    pub limits: Limits,
    pub task_sha256: String,
    pub task_bytes: usize,
    pub snapshot_manifest: BTreeMap<String, String>,
    pub snapshot_bytes: u64,
    /// `snapshot_read` pages needed to read every member in full (review P3-6; recorded).
    pub snapshot_read_pages: u64,
    pub secret_rules: String,
    pub frozen_world: FrozenWorld,
}

/// Everything one job needs from this crate.
#[derive(Debug, Clone)]
pub struct Bundle {
    pub record: JobInputs,
    pub requirements: Vec<u8>,
    pub contract: String,
    pub task: String,
    pub snapshot: Snapshot,
}

/// Composes the job inputs of `profile` over `requirements`, which must be the profile's revision
/// (a test passes the frozen requirements to see the refusal). Order: frozen-world outcome,
/// chosen-world preparation (refused on known incompatibility), freeze, revision identity, task
/// render (prompt bound), subject materialization bound to the preparation's `scope.digest`,
/// request-context fit.
pub fn build(
    profile: Profile,
    requirements: &[u8],
    repo: &Path,
    subject: &Subject,
    job_id: &str,
    limits: Limits,
) -> Result<Bundle, Refusal> {
    let frozen_world = revision::frozen_world_for(profile, subject)?;
    let frozen = revision::prepare_world_for(profile, requirements, subject, job_id)?;
    // Only the profile's own revision yields job inputs. On `claude-sdk` the frozen requirements
    // have no known incompatibility (they allow `anthropic`) and would otherwise freeze; their
    // conditions stay unmet and they never produce job inputs.
    if sha256_hex(requirements) != sha256_hex(profile.revision().requirements) {
        return Err(Refusal::Requirements(format!(
            "the requirements are not revision {} of profile {}",
            profile.revision().id,
            profile.provider()
        )));
    }
    let preparation = frozen.preparation();
    let task = task::render_for(profile, frozen.contract(), subject, limits)?;
    let snapshot = subject::materialize(
        repo,
        subject,
        &preparation.model_context.obligation.scope_digest,
    )?;
    subject::check_context_fit(snapshot.bytes(), task.len())?;
    subject::check_read_budget(&snapshot, limits.max_reads)?;
    let record = record(
        profile,
        requirements,
        &frozen,
        &task,
        limits,
        &snapshot,
        frozen_world,
    );
    Ok(Bundle {
        record,
        requirements: requirements.to_vec(),
        contract: frozen.contract().to_owned(),
        task,
        snapshot,
    })
}

/// The record of one job's inputs.
pub fn record(
    profile: Profile,
    requirements: &[u8],
    frozen: &FrozenDelivery,
    task: &str,
    limits: Limits,
    snapshot: &Snapshot,
    frozen_world: FrozenWorld,
) -> JobInputs {
    let preparation = frozen.preparation();
    let (template, _, template_sha256) = task::template(profile);
    JobInputs {
        schema: RECORD_SCHEMA.to_owned(),
        standing: RECORD_STANDING.to_owned(),
        profile,
        revision: profile.revision().id.to_owned(),
        requirements_sha256: sha256_hex(requirements),
        run_id: preparation.model_context.identity.run_id.clone(),
        subject: preparation.model_context.obligation.subject.clone(),
        parent: snapshot.parent.clone(),
        job_id: frozen.transport().attempt_id.clone(),
        transport: frozen.transport().clone(),
        requested_model: profile.model().to_owned(),
        provider: profile.provider().to_owned(),
        realization_claim: profile.claim(),
        blockers: preparation
            .blockers
            .iter()
            .map(|b| BlockerRef {
                id: b.id.clone(),
                class: serde_json::to_value(b.class)
                    .ok()
                    .and_then(|v| v.as_str().map(str::to_owned))
                    .unwrap_or_default(),
            })
            .collect(),
        contract_sha256: frozen.contract_sha256().to_owned(),
        contract_bytes: frozen.contract().len(),
        template: template.to_owned(),
        template_sha256: template_sha256.to_owned(),
        limits,
        task_sha256: sha256_hex(task.as_bytes()),
        task_bytes: task.len(),
        snapshot_manifest: snapshot.manifest(),
        snapshot_bytes: snapshot.bytes(),
        snapshot_read_pages: snapshot
            .members
            .values()
            .map(|b| subject::pages(b.len() as u64))
            .sum(),
        secret_rules: secrets::RULESET_ID.to_owned(),
        frozen_world,
    }
}

fn create_new(path: &Path, bytes: &[u8]) -> Result<(), Refusal> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Builds, then writes into `out` (which must not exist): `snapshot/`, `task.txt`,
/// `contract.json`, `requirements.json`, `frozen-world.json` and `job-inputs.json`. Files are
/// written into a sibling staging directory that is renamed to `out` only when every write
/// succeeded; on any refusal or I/O failure the staging directory is removed and `out` is not
/// created.
pub fn write(
    profile: Profile,
    requirements: &[u8],
    repo: &Path,
    subject: &Subject,
    job_id: &str,
    limits: Limits,
    out: &Path,
) -> Result<JobInputs, Refusal> {
    if out.exists() {
        return Err(Refusal::Io(format!("{} already exists", out.display())));
    }
    let bundle = build(profile, requirements, repo, subject, job_id, limits)?;
    write_bundle(&bundle, out)?;
    Ok(bundle.record)
}

/// Writes a built bundle through the staging directory (see [`write`]).
pub fn write_bundle(bundle: &Bundle, out: &Path) -> Result<(), Refusal> {
    if out.exists() {
        return Err(Refusal::Io(format!("{} already exists", out.display())));
    }
    let name = out
        .file_name()
        .ok_or_else(|| Refusal::Io(format!("{} has no file name", out.display())))?;
    let staging = out.with_file_name(format!(
        ".{}.staging-{}",
        name.to_string_lossy(),
        std::process::id()
    ));
    // Only a staging directory this call created is ever removed.
    std::fs::create_dir(&staging)?;
    let staged = (|| {
        bundle.snapshot.write(&staging.join("snapshot"))?;
        create_new(&staging.join("task.txt"), bundle.task.as_bytes())?;
        create_new(&staging.join("contract.json"), bundle.contract.as_bytes())?;
        create_new(&staging.join("requirements.json"), &bundle.requirements)?;
        create_new(
            &staging.join("frozen-world.json"),
            &pretty(&bundle.record.frozen_world)?,
        )?;
        create_new(&staging.join("job-inputs.json"), &pretty(&bundle.record)?)?;
        if out.exists() {
            return Err(Refusal::Io(format!("{} already exists", out.display())));
        }
        std::fs::rename(&staging, out)?;
        Ok(())
    })();
    if staged.is_err() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    staged
}

/// Pretty JSON plus a trailing newline.
pub fn pretty<T: Serialize>(value: &T) -> Result<Vec<u8>, Refusal> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|e| Refusal::Io(e.to_string()))?;
    bytes.push(b'\n');
    Ok(bytes)
}
