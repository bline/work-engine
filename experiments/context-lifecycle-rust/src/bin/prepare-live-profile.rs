//! Creates only an empty, pinned, disposable native profile. No login or inference.
use anyhow::{Result, ensure};
use context_lifecycle_experiment::codex::prepare_live_profile;
use std::path::Path;

fn main() -> Result<()> {
    let arguments: Vec<_> = std::env::args().collect();
    ensure!(
        arguments.len() == 2,
        "usage: prepare-live-profile ABSOLUTE_EMPTY_DIRECTORY"
    );
    let root = Path::new(&arguments[1]);
    prepare_live_profile(root)?;
    println!("{}", root.display());
    Ok(())
}
