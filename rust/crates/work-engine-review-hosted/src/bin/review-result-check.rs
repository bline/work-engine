//! Historical argument shape only. Current builds refuse rather than reinterpret a record.
//! For rechecks, use the exact archived source/checker pin and retained inputs.

use std::process::ExitCode;
use work_engine_review_hosted::inputs::JobInputs;
use work_engine_review_hosted::revision::archive_refusal;

const USAGE: &str =
    "expected --inputs <file> --run-inputs <dir> --job <file> --state-dir <dir> --out <file>";

fn run() -> Result<(), String> {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let [k1, inputs, k2, _, k3, _, k4, _, k5, _] = &raw[..] else {
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
    Err(format!(
        "refused: {}",
        archive_refusal(record.profile, &record.revision)
    ))
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(text) => {
            eprintln!("{text}");
            ExitCode::from(2)
        }
    }
}
