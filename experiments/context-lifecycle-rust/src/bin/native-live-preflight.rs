//! Explicit parent-run read-only account/config/catalog check; no inference turn.
use anyhow::{Result, ensure};
use context_lifecycle_experiment::codex::preflight_live_profile;
use std::path::Path;

fn main() -> Result<()> {
    let arguments: Vec<_> = std::env::args().collect();
    ensure!(
        arguments.len() == 2,
        "usage: native-live-preflight PREPARED_PROFILE_DIRECTORY"
    );
    let result = preflight_live_profile(Path::new(&arguments[1]))?;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
