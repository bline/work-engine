//! The `work-engine-evaluation` command line (contract 2.2).
//!
//! `check --kind estimate-set FILE` prints the report as `to_string_pretty` plus one newline on
//! stdout. There is no option that names a packet or projection path: the pointers stored in
//! the file are the only ones resolved (contract S6). A usage, IO or parse failure writes one
//! JSON object `{error, errors, usage}` to stderr and nothing to stdout.
//!
//! `compare` derives a comparison finding (contract 3.2). Exit codes: 0 valid, 1 invalid, 2 usage,
//! IO, parse or path failure, 3 a binding refusal by `compare` (nothing is written).

use crate::check::{KIND, KIND_CONTRACT, check_contract_bytes, check_estimate_set_bytes};
use crate::compare::compare_paths;
use crate::diag::{EXIT_INVALID, EXIT_OK, Failure};
use serde::Serialize;
use std::io::Write;

const USAGE: &str = "usage:\n  work-engine-evaluation check --kind estimate-set FILE\n  work-engine-evaluation check --kind contract FILE\n  work-engine-evaluation compare --contract C --estimates E [--estimates E ...] --derived-at T [--out F]";

/// Serialise an output document: `to_string_pretty` plus one `\n` (contract S2).
pub fn to_output_string<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    Ok(text)
}

/// Run the CLI. Returns the process exit code.
pub fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match dispatch(args, out) {
        Ok(code) => code,
        Err(failure) => {
            let body = serde_json::json!({
                "error": failure.to_string(),
                "errors": failure.errors(),
                "usage": USAGE,
            });
            let _ = writeln!(err, "{body}");
            failure.exit_code()
        }
    }
}

fn dispatch(args: &[String], out: &mut dyn Write) -> Result<i32, Failure> {
    match args.first().map(String::as_str) {
        Some("check") => check(&args[1..], out),
        Some("compare") => compare(&args[1..], out),
        Some(other) => Err(Failure::Usage(format!("unknown subcommand {other:?}"))),
        None => Err(Failure::Usage("no subcommand given".to_string())),
    }
}

/// `--kind K FILE`: one `--kind`, one positional; any other option or repetition is refused.
fn parse_check(args: &[String]) -> Result<(String, String), Failure> {
    let mut kind: Option<String> = None;
    let mut files: Vec<&String> = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.strip_prefix("--") {
            Some("kind") => {
                let value = iter
                    .next()
                    .ok_or_else(|| Failure::Usage("--kind needs a value".to_string()))?;
                if kind.replace(value.clone()).is_some() {
                    return Err(Failure::Usage("--kind given more than once".to_string()));
                }
            }
            Some(name) => return Err(Failure::Usage(format!("unexpected option --{name}"))),
            None => files.push(arg),
        }
    }
    let kind = kind.ok_or_else(|| Failure::Usage("check needs --kind".to_string()))?;
    match files.as_slice() {
        [file] => Ok((kind, (*file).clone())),
        [] => Err(Failure::Usage("check needs a FILE".to_string())),
        _ => Err(Failure::Usage("check takes exactly one FILE".to_string())),
    }
}

fn check(args: &[String], out: &mut dyn Write) -> Result<i32, Failure> {
    let (kind, file) = parse_check(args)?;
    if kind != KIND && kind != KIND_CONTRACT {
        return Err(Failure::Usage(format!(
            "unknown --kind {kind:?}; this build checks {KIND:?} and {KIND_CONTRACT:?}"
        )));
    }
    let bytes = std::fs::read(&file).map_err(|e| Failure::Io {
        path: file.clone(),
        message: e.to_string(),
    })?;
    let report = if kind == KIND_CONTRACT {
        check_contract_bytes(&bytes)?
    } else {
        check_estimate_set_bytes(&bytes)?
    };
    let text = to_output_string(&report)
        .map_err(|e| Failure::Usage(format!("cannot serialise output: {e}")))?;
    out.write_all(text.as_bytes())
        .map_err(|e| Failure::Usage(format!("cannot write output: {e}")))?;
    Ok(if report.valid { EXIT_OK } else { EXIT_INVALID })
}

/// `compare --contract C --estimates E... --derived-at T [--out F]` (contract 3.2). Exit 0 with
/// the finding on stdout (or in F, which must not exist); exit 2 for a relative path or a usage
/// error; exit 3 for a binding refusal or an input that fails its own check. Nothing is written
/// unless the derivation succeeded.
fn compare(args: &[String], out: &mut dyn Write) -> Result<i32, Failure> {
    let (mut contract, mut estimates, mut at, mut dest) = (None, Vec::new(), None, None);
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = |name: &str| {
            iter.next()
                .cloned()
                .ok_or_else(|| Failure::Usage(format!("{name} needs a value")))
        };
        match arg.as_str() {
            "--contract" => set_once(&mut contract, value("--contract")?, "--contract")?,
            "--estimates" => estimates.push(value("--estimates")?),
            "--derived-at" => set_once(&mut at, value("--derived-at")?, "--derived-at")?,
            "--out" => set_once(&mut dest, value("--out")?, "--out")?,
            other => return Err(Failure::Usage(format!("unexpected argument {other:?}"))),
        }
    }
    let contract =
        contract.ok_or_else(|| Failure::Usage("compare needs --contract".to_string()))?;
    let at = at.ok_or_else(|| Failure::Usage("compare needs --derived-at".to_string()))?;
    let finding = compare_paths(&contract, &estimates, &at)?;
    let text = to_output_string(&finding)
        .map_err(|e| Failure::Usage(format!("cannot serialise output: {e}")))?;
    match dest {
        Some(path) => {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|e| Failure::Io {
                    path: path.clone(),
                    message: format!("{e} (--out refuses to overwrite)"),
                })?;
            file.write_all(text.as_bytes()).map_err(|e| Failure::Io {
                path,
                message: e.to_string(),
            })?;
        }
        None => out
            .write_all(text.as_bytes())
            .map_err(|e| Failure::Usage(format!("cannot write output: {e}")))?,
    }
    Ok(EXIT_OK)
}

fn set_once(slot: &mut Option<String>, value: String, name: &str) -> Result<(), Failure> {
    if slot.replace(value).is_some() {
        return Err(Failure::Usage(format!("{name} given more than once")));
    }
    Ok(())
}
