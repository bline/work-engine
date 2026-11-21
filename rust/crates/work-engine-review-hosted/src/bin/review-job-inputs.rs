//! Historical argument shape only. Current builds refuse both archival profiles.
//!
//! ```text
//! review-job-inputs --provider <openai-codex|claude-sdk> --repo <path> --commit <sha>
//!                   --tree <sha> --patch-identity <sha256> --job-id <id> --max-requests <n>
//!                   --max-reads <n> --out <new dir>
//! ```
//!
//! The provider is one of the closed set of profiles; no other provider or model is accepted.

use std::collections::BTreeMap;
use std::process::ExitCode;
use work_engine_review_hosted::revision::{Profile, archive_refusal};

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

fn run() -> Result<(), String> {
    let a = args()?;
    let profile = Profile::parse(&a["--provider"]).ok_or_else(|| {
        let names: Vec<&str> = Profile::ALL.iter().map(|p| p.provider()).collect();
        format!("--provider must be one of {names:?}")
    })?;
    Err(format!(
        "refused: {}",
        archive_refusal(profile, profile.revision().id)
    ))
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(text) => {
            eprintln!("{text}");
            ExitCode::FAILURE
        }
    }
}
