//! Checks one hosted review job's answer and writes the non-consumable envelope.
//!
//! ```text
//! review-result-check --inputs <job-inputs.json> --run-inputs <base>/inputs --job <base>/job.json
//!                     --state-dir <base>/state --out <new envelope.json>
//! ```
//!
//! `<base>` is the run directory `analyze.py` wrote; its job id is
//! `dev-vertical-hosted-analysis-<run name>` (`analyze.py:88`), which `review-job-inputs --job-id`
//! must have been given.
//!
//! Exit 0: every check passed (the envelope is still `owner_pending`, not consumable). Exit 1: the
//! envelope records a failed check. Exit 2: no envelope (unreadable inputs or usage).

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use work_engine_review_hosted::check::{check, load_run, load_state};
use work_engine_review_hosted::inputs::{JobInputs, pretty};

const USAGE: &str =
    "expected --inputs <file> --run-inputs <dir> --job <file> --state-dir <dir> --out <file>";

fn run() -> Result<bool, String> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let [k1, inputs, k2, run_inputs, k3, job, k4, state, k5, out] = &raw[..] else {
        return Err(USAGE.into());
    };
    if [k1, k2, k3, k4, k5].map(String::as_str)
        != ["--inputs", "--run-inputs", "--job", "--state-dir", "--out"]
    {
        return Err(USAGE.into());
    }
    let record: JobInputs =
        serde_json::from_slice(&std::fs::read(inputs).map_err(|e| e.to_string())?)
            .map_err(|e| format!("job inputs: {e}"))?;
    let run =
        load_run(&PathBuf::from(run_inputs), &PathBuf::from(job)).map_err(|e| e.to_string())?;
    let (result, requests) = load_state(&PathBuf::from(state)).map_err(|e| e.to_string())?;
    let envelope = check(&record, &run, &result, &requests);
    let bytes = pretty(&envelope).map_err(|e| e.to_string())?;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out)
        .and_then(|mut f| f.write_all(&bytes))
        .map_err(|e| format!("{out}: {e}"))?;
    for c in &envelope.checks {
        println!("{}: {:?}", c.id, c.outcome);
    }
    println!(
        "outcome: {} (standing {}, consumable {})",
        envelope.outcome, envelope.standing, envelope.consumable
    );
    Ok(envelope.outcome == "checks_passed")
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(text) => {
            eprintln!("{text}");
            ExitCode::from(2)
        }
    }
}
