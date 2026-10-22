//! The `work-engine-readiness` command line (contract 2.2).
//!
//! Output is JSON on stdout, or a file named by `--out`, which refuses to overwrite.
//! A refusal or an input failure writes one JSON object to stderr and nothing to stdout.
//!
//! Exit codes: 0 success, 1 `check` found the document invalid, 2 usage, IO or parse
//! error (including a refused `plan`), 3 a binding error (`evaluate`, `recheck`).
//! This build provides `check --kind profile|subject|observation` and `plan`.

use crate::diag::{CheckReport, ErrorItem, codes};
use crate::digest::sha256_hex;
use crate::model::*;
use crate::plan::{PairBytes, PlanError, derive_plan, plan_to_string};
use crate::validate::{
    DocError, parse_document, validate_observation, validate_profile, validate_subject,
};
use serde::Serialize;
use std::io::Write;

pub const EXIT_OK: i32 = 0;
pub const EXIT_INVALID: i32 = 1;
pub const EXIT_USAGE: i32 = 2;

const USAGE: &str = "usage:\n  work-engine-readiness check --kind profile|subject|observation FILE [--profile P]\n  work-engine-readiness plan (--profile P --subject S)+ [--out FILE]\n(`subject` requires --profile; `plan` pairs the nth --profile with the nth --subject)";

/// Serialise any output document: `to_string_pretty` plus one `\n`.
pub fn to_output_string<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    Ok(text)
}

pub struct Failure {
    pub message: String,
    pub errors: Vec<ErrorItem>,
}

impl Failure {
    fn usage(message: impl Into<String>) -> Self {
        Failure {
            message: message.into(),
            errors: Vec::new(),
        }
    }
}

fn read_file(path: &str) -> Result<Vec<u8>, Failure> {
    std::fs::read(path).map_err(|error| Failure::usage(format!("cannot read {path}: {error}")))
}

/// Run the CLI. Returns the process exit code.
pub fn run(args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match dispatch(args, out) {
        Ok(code) => code,
        Err(failure) => {
            let body = serde_json::json!({
                "error": failure.message,
                "errors": failure.errors,
                "usage": USAGE,
            });
            let _ = writeln!(err, "{body}");
            EXIT_USAGE
        }
    }
}

fn dispatch(args: &[String], out: &mut dyn Write) -> Result<i32, Failure> {
    match args.first().map(String::as_str) {
        Some("check") => check(&args[1..], out),
        Some("plan") => plan(&args[1..], out),
        Some(other) => Err(Failure::usage(format!("unknown subcommand {other:?}"))),
        None => Err(Failure::usage("no subcommand given")),
    }
}

/// Parsed options: repeated `--name value` pairs and positional arguments.
struct Options {
    named: Vec<(String, String)>,
    positional: Vec<String>,
}

impl Options {
    fn parse(args: &[String], allowed: &[&str]) -> Result<Self, Failure> {
        let mut named = Vec::new();
        let mut positional = Vec::new();
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if let Some(name) = arg.strip_prefix("--") {
                if !allowed.contains(&name) {
                    return Err(Failure::usage(format!("unexpected option --{name}")));
                }
                let value = iter
                    .next()
                    .ok_or_else(|| Failure::usage(format!("--{name} needs a value")))?;
                named.push((name.to_string(), value.clone()));
            } else {
                positional.push(arg.clone());
            }
        }
        Ok(Options { named, positional })
    }

    fn all(&self, name: &str) -> Vec<&str> {
        self.named
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
            .collect()
    }

    fn one(&self, name: &str) -> Result<Option<&str>, Failure> {
        let values = self.all(name);
        match values.as_slice() {
            [] => Ok(None),
            [single] => Ok(Some(single)),
            _ => Err(Failure::usage(format!("--{name} given more than once"))),
        }
    }
}

fn emit(out: &mut dyn Write, text: &str) -> Result<(), Failure> {
    out.write_all(text.as_bytes())
        .map_err(|error| Failure::usage(format!("cannot write output: {error}")))
}

fn serialise_failure(error: serde_json::Error) -> Failure {
    Failure::usage(format!("cannot serialise output: {error}"))
}

fn schema_report(kind: &str, message: String) -> CheckReport {
    CheckReport::new(kind, vec![ErrorItem::new(codes::SCHEMA, "", message)])
}

fn check(args: &[String], out: &mut dyn Write) -> Result<i32, Failure> {
    let options = Options::parse(args, &["kind", "profile"])?;
    let kind = options
        .one("kind")?
        .ok_or_else(|| Failure::usage("check needs --kind"))?;
    let [file] = options.positional.as_slice() else {
        return Err(Failure::usage("check takes exactly one FILE"));
    };
    let profile_arg = options.one("profile")?;
    let report = match kind {
        "profile" => {
            if profile_arg.is_some() {
                return Err(Failure::usage("--profile does not apply to --kind profile"));
            }
            check_profile_bytes(&read_file(file)?)?
        }
        "observation" => {
            if profile_arg.is_some() {
                return Err(Failure::usage(
                    "--profile does not apply to --kind observation",
                ));
            }
            check_observation_bytes(&read_file(file)?)?
        }
        "subject" => {
            let profile_path = profile_arg
                .ok_or_else(|| Failure::usage("check --kind subject needs --profile"))?;
            check_subject_bytes(&read_file(profile_path)?, &read_file(file)?)?
        }
        "evaluation" | "assessment" => {
            return Err(Failure::usage(format!(
                "check --kind {kind} is not available in this build"
            )));
        }
        other => return Err(Failure::usage(format!("unknown --kind {other:?}"))),
    };
    emit(out, &to_output_string(&report).map_err(serialise_failure)?)?;
    Ok(if report.valid { EXIT_OK } else { EXIT_INVALID })
}

/// `check --kind profile` on file bytes.
pub fn check_profile_bytes(bytes: &[u8]) -> Result<CheckReport, Failure> {
    match parse_document::<Profile>(bytes) {
        Ok(profile) => Ok(CheckReport::new("profile", validate_profile(&profile))),
        Err(DocError::Schema(message)) => Ok(schema_report("profile", message)),
        Err(DocError::Syntax(message)) => Err(Failure::usage(format!("profile: {message}"))),
    }
}

/// `check --kind observation` on file bytes.
pub fn check_observation_bytes(bytes: &[u8]) -> Result<CheckReport, Failure> {
    match parse_document::<Observation>(bytes) {
        Ok(observation) => Ok(CheckReport::new(
            "observation",
            validate_observation(&observation),
        )),
        Err(DocError::Schema(message)) => Ok(schema_report("observation", message)),
        Err(DocError::Syntax(message)) => Err(Failure::usage(format!("observation: {message}"))),
    }
}

/// `check --kind subject --profile P` on file bytes. The manifests the subject names are
/// read from the filesystem, as `plan` reads them.
pub fn check_subject_bytes(
    profile_bytes: &[u8],
    subject_bytes: &[u8],
) -> Result<CheckReport, Failure> {
    let profile = match parse_document::<Profile>(profile_bytes) {
        Ok(profile) => profile,
        Err(error) => return Err(Failure::usage(format!("--profile does not parse: {error}"))),
    };
    let profile_errors = validate_profile(&profile);
    if !profile_errors.is_empty() {
        return Err(Failure {
            message: "--profile is not a valid profile; run check --kind profile".to_string(),
            errors: profile_errors,
        });
    }
    let subject = match parse_document::<Subject>(subject_bytes) {
        Ok(subject) => subject,
        Err(DocError::Schema(message)) => return Ok(schema_report("subject", message)),
        Err(DocError::Syntax(message)) => {
            return Err(Failure::usage(format!("subject: {message}")));
        }
    };
    let read = |path: &str| std::fs::read(path);
    let outcome = validate_subject(&profile, &sha256_hex(profile_bytes), &subject, &read)
        .map_err(|error| Failure::usage(error.to_string()))?;
    Ok(CheckReport::new("subject", outcome.errors))
}

fn plan(args: &[String], out: &mut dyn Write) -> Result<i32, Failure> {
    let options = Options::parse(args, &["profile", "subject", "out"])?;
    if !options.positional.is_empty() {
        return Err(Failure::usage("plan takes no positional arguments"));
    }
    let profiles = options.all("profile");
    let subjects = options.all("subject");
    if profiles.is_empty() || profiles.len() != subjects.len() {
        return Err(Failure::usage(
            "plan needs one or more --profile P --subject S pairs, with equal counts",
        ));
    }
    let out_path = options.one("out")?;
    let mut contents = Vec::with_capacity(profiles.len());
    for (profile, subject) in profiles.iter().zip(subjects.iter()) {
        contents.push((read_file(profile)?, read_file(subject)?));
    }
    let pairs: Vec<PairBytes<'_>> = contents
        .iter()
        .map(|(profile, subject)| PairBytes { profile, subject })
        .collect();
    let read = |path: &str| std::fs::read(path);
    let plan = derive_plan(&pairs, &read).map_err(|error| match error {
        PlanError::Input(message) => Failure::usage(message),
        PlanError::Refused(errors) => Failure {
            message: format!("plan refused: {} violated rule(s)", errors.len()),
            errors,
        },
    })?;
    let text = plan_to_string(&plan).map_err(serialise_failure)?;
    match out_path {
        Some(path) => {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|error| Failure::usage(format!("cannot create {path}: {error}")))?;
            file.write_all(text.as_bytes())
                .map_err(|error| Failure::usage(format!("cannot write {path}: {error}")))?;
        }
        None => emit(out, &text)?,
    }
    Ok(EXIT_OK)
}
