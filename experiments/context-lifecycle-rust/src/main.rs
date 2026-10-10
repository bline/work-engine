use anyhow::{Result, bail};
use context_lifecycle_experiment::{Stage, process};
use std::path::Path;
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("codex-provider") if args.len() == 3 => {
            let launch = serde_json::from_slice(&std::fs::read(&args[2])?)?;
            context_lifecycle_experiment::codex::run_provider(launch)
        }
        Some("provider") if args.len() == 3 => process::run_provider(Path::new(&args[2])),
        Some("coordinator") if (4..=5).contains(&args.len()) => {
            let pause = match args.get(4).map(String::as_str) {
                None => None,
                Some("before-entry") => Some(Stage::BeforeEntry),
                Some("after-entry") => Some(Stage::AfterEntry),
                Some("after-send") => Some(Stage::AfterSend),
                Some("after-result") => Some(Stage::AfterResult),
                _ => bail!("unknown barrier"),
            };
            process::run_coordinator(Path::new(&args[2]), args[3].parse()?, pause)
        }
        Some("synthetic-worker-exit") => Ok(()),
        _ => bail!(
            "usage: context-lifecycle-experiment provider DB | coordinator DB PROVIDER_ADDR [before-entry|after-entry|after-send|after-result] | synthetic-worker-exit"
        ),
    }
}
