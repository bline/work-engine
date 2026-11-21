//! Job artifact shape (including historical syntax), and the private offline writer.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[cfg(test)]
use std::path::Path;
#[cfg(test)]
use work_engine_review_delivery::FrozenDelivery;
use work_engine_review_delivery::Transport;
use work_engine_review_preparation::{RealizationClaim, Subject};

use crate::Refusal;
#[cfg(test)]
use crate::offline::ExpectedWorld;
#[cfg(test)]
use crate::revision;
use crate::revision::{FrozenWorld, Profile};
#[cfg(test)]
use crate::subject::{self, Snapshot};
#[cfg(test)]
use crate::task;
use crate::task::Limits;
#[cfg(test)]
use crate::{secrets, sha256_hex};

#[cfg(test)]
pub(crate) const RECORD_SCHEMA: &str = "work-engine.review-hosted-offline-exact-inputs.v1";
#[cfg(test)]
pub(crate) const RECORD_STANDING: &str = "synthetic_inputs_only_not_an_admission";

/// One blocker of the chosen world's preparation, as recorded (none is discharged).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlockerRef {
    pub id: String,
    pub class: String,
}

/// The record that binds the requirements revision, preparation, contract, task, snapshot and
/// frozen-world counterfactual of one job. This is data, not evidence or permission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobInputs {
    pub schema: String,
    pub standing: String,
    /// The provider/wire profile, independent of the requirements revision.
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

/// Artifacts produced by the private offline writer.
#[cfg(test)]
#[derive(Debug, Clone)]
pub(crate) struct Bundle {
    pub record: JobInputs,
    pub requirements: Vec<u8>,
    pub contract: String,
    pub task: String,
    pub snapshot: Snapshot,
}

/// Only an internally pinned world can produce an offline bundle. Preparation can
/// refuse before identity verification (as for the frozen Responses counterfactual).
#[cfg(test)]
pub(crate) fn build(
    expected: &ExpectedWorld,
    repo: &Path,
    subject: &Subject,
    job_id: &str,
    limits: Limits,
) -> Result<Bundle, Refusal> {
    let profile = expected.profile;
    let requirements = expected.requirements();
    let frozen_world = revision::frozen_world_for(profile, subject)?;
    let frozen = revision::prepare_world_for(profile, requirements, subject, job_id)?;
    expected.verify_bytes()?;
    if frozen.preparation().model_context.identity.run_id != expected.run_id {
        return Err(Refusal::Requirements(format!(
            "expected run id {}",
            expected.run_id
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
    let record = record(expected, &frozen, &task, limits, &snapshot, frozen_world);
    Ok(Bundle {
        record,
        requirements: requirements.to_vec(),
        contract: frozen.contract().to_owned(),
        task,
        snapshot,
    })
}

/// Records input data, not a preparation or materialization verdict.
#[cfg(test)]
pub(crate) fn record(
    expected: &ExpectedWorld,
    frozen: &FrozenDelivery,
    task: &str,
    limits: Limits,
    snapshot: &Snapshot,
    frozen_world: FrozenWorld,
) -> JobInputs {
    let profile = expected.profile;
    let preparation = frozen.preparation();
    let (template, _, template_sha256) = task::template(profile);
    JobInputs {
        schema: RECORD_SCHEMA.to_owned(),
        standing: RECORD_STANDING.to_owned(),
        profile,
        revision: expected.id.to_owned(),
        requirements_sha256: sha256_hex(expected.requirements()),
        run_id: preparation.model_context.identity.run_id.clone(),
        subject: preparation.model_context.obligation.subject.clone(),
        parent: snapshot.parent.clone(),
        job_id: frozen.transport().attempt_id.clone(),
        transport: frozen.transport().clone(),
        requested_model: profile.model().to_owned(),
        provider: profile.provider().to_owned(),
        realization_claim: profile.claim(),
        blockers: blocker_refs(preparation),
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

#[cfg(test)]
pub(crate) fn blocker_refs(
    preparation: &work_engine_review_preparation::ReviewPreparation,
) -> Vec<BlockerRef> {
    use work_engine_review_preparation::BlockerClass;
    preparation
        .blockers
        .iter()
        .map(|b| BlockerRef {
            id: b.id.clone(),
            class: match b.class {
                BlockerClass::KnownIncompatibility => "known_incompatibility",
                BlockerClass::Unestablished => "unestablished",
            }
            .to_owned(),
        })
        .collect()
}

#[cfg(test)]
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
#[cfg(test)]
pub(crate) fn write(
    expected: &ExpectedWorld,
    repo: &Path,
    subject: &Subject,
    job_id: &str,
    limits: Limits,
    out: &Path,
) -> Result<JobInputs, Refusal> {
    if out.exists() {
        return Err(Refusal::Io(format!("{} already exists", out.display())));
    }
    let bundle = build(expected, repo, subject, job_id, limits)?;
    write_bundle(&bundle, out)?;
    Ok(bundle.record)
}

/// Writes a built bundle through the staging directory (see [`write`]).
#[cfg(test)]
pub(crate) fn write_bundle(bundle: &Bundle, out: &Path) -> Result<(), Refusal> {
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
