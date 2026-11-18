//! Writes the job inputs of one hosted review job for an explicitly chosen provider profile over
//! that profile's requirements revision (`openai-codex`: `hosted-route-qualification-r1`;
//! `claude-sdk`: `hosted-route-qualification-r2`). Refuses, writing nothing, on any check failure.
//!
//! ```text
//! review-job-inputs --provider <openai-codex|claude-sdk> --repo <path> --commit <sha>
//!                   --tree <sha> --patch-identity <sha256> --job-id <id> --max-requests <n>
//!                   --max-reads <n> --out <new dir>
//! ```
//!
//! The provider is one of the closed set of profiles; no other provider or model is accepted.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::ExitCode;
use work_engine_review_hosted::revision::Profile;
use work_engine_review_hosted::task::Limits;
use work_engine_review_hosted::{inputs, sha256_hex};
use work_engine_review_preparation::Subject;

const KEYS: [&str; 9] = [
    "--provider",
    "--repo",
    "--commit",
    "--tree",
    "--patch-identity",
    "--job-id",
    "--max-requests",
    "--max-reads",
    "--out",
];

fn args() -> Result<BTreeMap<String, String>, String> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.len() != KEYS.len() * 2 {
        return Err(format!("expected exactly {}", KEYS.join(" ")));
    }
    let mut map = BTreeMap::new();
    for pair in raw.chunks(2) {
        if !KEYS.contains(&pair[0].as_str())
            || map.insert(pair[0].clone(), pair[1].clone()).is_some()
        {
            return Err(format!("unexpected or repeated argument {}", pair[0]));
        }
    }
    Ok(map)
}

fn run() -> Result<String, String> {
    let a = args()?;
    let profile = Profile::parse(&a["--provider"]).ok_or_else(|| {
        let names: Vec<&str> = Profile::ALL.iter().map(|p| p.provider()).collect();
        format!("--provider must be one of {names:?}")
    })?;
    let number = |k: &str| {
        a[k].parse::<u32>()
            .map_err(|_| format!("{k} must be a number"))
    };
    let subject = Subject {
        commit: a["--commit"].clone(),
        tree: a["--tree"].clone(),
        patch_identity: a["--patch-identity"].clone(),
    };
    let limits = Limits {
        max_requests: number("--max-requests")?,
        max_reads: number("--max-reads")?,
    };
    let out = PathBuf::from(&a["--out"]);
    let record = inputs::write(
        profile,
        profile.revision().requirements,
        &PathBuf::from(&a["--repo"]),
        &subject,
        &a["--job-id"],
        limits,
        &out,
    )
    .map_err(|e| format!("refused: {e}"))?;
    let record_bytes = std::fs::read(out.join("job-inputs.json")).map_err(|e| e.to_string())?;
    Ok(format!(
        "job_inputs: {}\njob_inputs_sha256: {}\nprofile: {} / {} (revision {})\ntask_sha256: {}\ncontract_sha256: {}\nsnapshot: {} ({} members, {} bytes)\nfrozen_world: {} ({} known incompatibilities, {} unestablished confinement)\nstanding: {}",
        out.join("job-inputs.json").display(),
        sha256_hex(&record_bytes),
        record.provider,
        record.requested_model,
        record.revision,
        record.task_sha256,
        record.contract_sha256,
        out.join("snapshot").display(),
        record.snapshot_manifest.len(),
        record.snapshot_bytes,
        record.frozen_world.outcome,
        record.frozen_world.known_incompatibilities.len(),
        record.frozen_world.unestablished_confinement.len(),
        record.standing,
    ))
}

fn main() -> ExitCode {
    match run() {
        Ok(text) => {
            println!("{text}");
            ExitCode::SUCCESS
        }
        Err(text) => {
            eprintln!("{text}");
            ExitCode::FAILURE
        }
    }
}
