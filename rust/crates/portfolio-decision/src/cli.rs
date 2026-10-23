//! The `work-engine-portfolio` command line (contract 4.2).
//!
//! `check --kind portfolio-decision FILE` prints the report as `to_string_pretty` plus one
//! newline on stdout. No option names a packet, decision, comparison or assessment path: the
//! pointers stored in the file are the only ones resolved (contract S6). A usage, IO, parse or
//! path failure writes one JSON object `{error, errors, usage}` to stderr and nothing to stdout.
//!
//! Exit codes: 0 valid, 1 invalid, 2 usage, IO, parse or path failure. There is no exit 3: this
//! crate has no derive command, and a digest refusal on a bound file is an ordinary error of the
//! report (`digest_mismatch`), not a refusal to run.

use crate::check::{KIND_PORTFOLIO, check_portfolio_bytes};
use crate::diag::{EXIT_INVALID, EXIT_OK, ErrorItem, Failure, codes};
use crate::read::read_stored;
use proposal_evaluation::validate::is_normalised_absolute;
use serde::Serialize;
use std::io::Write;

const USAGE: &str = "usage:\n  work-engine-portfolio check --kind portfolio-decision FILE";

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
    if kind != KIND_PORTFOLIO {
        return Err(Failure::Usage(format!(
            "unknown --kind {kind:?}; this build checks {KIND_PORTFOLIO:?}"
        )));
    }
    if !is_normalised_absolute(&file) {
        return Err(Failure::Refused {
            message: "the FILE path is not absolute and normalised".to_string(),
            errors: vec![ErrorItem::new(
                codes::PATH_NOT_ABSOLUTE,
                "FILE",
                format!("{file:?} must be absolute and normalised"),
            )],
        });
    }
    let bytes = read_stored(&file).map_err(|e| Failure::Io {
        path: file.clone(),
        message: e.to_string(),
    })?;
    let report = check_portfolio_bytes(&bytes)?;
    let text = to_output_string(&report)
        .map_err(|e| Failure::Usage(format!("cannot serialise output: {e}")))?;
    out.write_all(text.as_bytes())
        .map_err(|e| Failure::Usage(format!("cannot write output: {e}")))?;
    Ok(if report.valid { EXIT_OK } else { EXIT_INVALID })
}
