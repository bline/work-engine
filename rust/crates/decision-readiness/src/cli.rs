//! The `work-engine-readiness` command line (contract 2.2).
//!
//! Output is JSON on stdout, or a file named by `--out`, which refuses to overwrite.
//! A refusal or an input failure writes one JSON object to stderr and nothing to stdout.
//!
//! Exit codes: 0 success, 1 `check` found the document invalid, 2 usage, IO or parse
//! error (including a refused `plan`), 3 a binding error (`evaluate`, `recheck`).
//! This build provides `check --kind profile|subject|observation|evaluation|assessment`, `plan`,
//! `evaluate` and `recheck`. It has no subcommand that writes an assessment.

use crate::check::{
    AssessmentContext, changed_checks, check_assessment, check_codes as cc, check_evaluation,
    check_evaluation_classes, compare_evaluations, raw_purpose,
};
use crate::diag::{CheckReport, ErrorItem, codes};
use crate::digest::sha256_hex;
use crate::evaluate::{
    EvalError, EvaluateInputs, Recorded, binding_codes as bc, evaluate as run_evaluate,
};
use crate::model::*;
use crate::plan::{PairBytes, PlanError, derive_plan, plan_to_string};
use crate::validate::{
    DocError, is_normalised_absolute, parse_document, validate_observation, validate_profile,
    validate_subject,
};
use serde::Serialize;
use serde_json::Value;
use std::io::Write;

pub const EXIT_OK: i32 = 0;
pub const EXIT_INVALID: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_BINDING: i32 = 3;

const USAGE: &str = "usage:\n  work-engine-readiness check --kind profile|observation FILE\n  work-engine-readiness check --kind subject FILE --profile P\n  work-engine-readiness check --kind evaluation FILE [--rederive --profile P --subject S]\n  work-engine-readiness check --kind assessment FILE --evaluation E --profile P\n  work-engine-readiness plan (--profile P --subject S)+ [--out FILE]\n  work-engine-readiness evaluate --profile P --subject S --run-plan RP --battery R --observation O --evaluated-at YYYY-MM-DDTHH:MM:SSZ [--out FILE]\n  work-engine-readiness recheck --evaluation E --battery R2 --observation O2 [--out FILE]\n(`subject` requires --profile; `plan` pairs the nth --profile with the nth --subject)";

/// Serialise any output document: `to_string_pretty` plus one `\n`.
pub fn to_output_string<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    Ok(text)
}

pub struct Failure {
    pub message: String,
    pub errors: Vec<ErrorItem>,
    /// The process exit code: 2 (usage, IO, parse, a refused document) or 3 (a binding refusal).
    pub code: i32,
}

impl Failure {
    fn usage(message: impl Into<String>) -> Self {
        Failure {
            message: message.into(),
            errors: Vec::new(),
            code: EXIT_USAGE,
        }
    }

    fn refused(message: impl Into<String>, errors: Vec<ErrorItem>) -> Self {
        Failure {
            message: message.into(),
            errors,
            code: EXIT_USAGE,
        }
    }

    fn binding(message: impl Into<String>, errors: Vec<ErrorItem>) -> Self {
        Failure {
            message: message.into(),
            errors,
            code: EXIT_BINDING,
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
            failure.code
        }
    }
}

fn dispatch(args: &[String], out: &mut dyn Write) -> Result<i32, Failure> {
    match args.first().map(String::as_str) {
        Some("check") => check(&args[1..], out),
        Some("plan") => plan(&args[1..], out),
        Some("evaluate") => evaluate(&args[1..], out),
        Some("recheck") => recheck(&args[1..], out),
        Some(other) => Err(Failure::usage(format!("unknown subcommand {other:?}"))),
        None => Err(Failure::usage("no subcommand given")),
    }
}

/// Parsed options: repeated `--name value` pairs, boolean `--flag`s and positional arguments.
struct Options {
    named: Vec<(String, String)>,
    flags: Vec<String>,
    positional: Vec<String>,
}

impl Options {
    fn parse(args: &[String], allowed: &[&str]) -> Result<Self, Failure> {
        Self::parse_with_flags(args, allowed, &[])
    }

    /// `flags` name options that take no value (`--rederive`).
    fn parse_with_flags(
        args: &[String],
        allowed: &[&str],
        flags: &[&str],
    ) -> Result<Self, Failure> {
        let mut named = Vec::new();
        let mut flag_set = Vec::new();
        let mut positional = Vec::new();
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            if let Some(name) = arg.strip_prefix("--") {
                if flags.contains(&name) {
                    if flag_set.iter().any(|f| f == name) {
                        return Err(Failure::usage(format!("--{name} given more than once")));
                    }
                    flag_set.push(name.to_string());
                    continue;
                }
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
        Ok(Options {
            named,
            flags: flag_set,
            positional,
        })
    }

    fn flag(&self, name: &str) -> bool {
        self.flags.iter().any(|f| f == name)
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
    let options = Options::parse_with_flags(
        args,
        &["kind", "profile", "subject", "evaluation"],
        &["rederive"],
    )?;
    let kind = options
        .one("kind")?
        .ok_or_else(|| Failure::usage("check needs --kind"))?;
    let [file] = options.positional.as_slice() else {
        return Err(Failure::usage("check takes exactly one FILE"));
    };
    let profile_arg = options.one("profile")?;
    let subject_arg = options.one("subject")?;
    let evaluation_arg = options.one("evaluation")?;
    let rederive = options.flag("rederive");
    let allowed: &[&str] = match kind {
        "profile" | "observation" => &[],
        "subject" => &["profile"],
        "evaluation" => &["profile", "subject", "rederive"],
        "assessment" => &["profile", "evaluation"],
        other => return Err(Failure::usage(format!("unknown --kind {other:?}"))),
    };
    for (name, present) in [
        ("profile", profile_arg.is_some()),
        ("subject", subject_arg.is_some()),
        ("evaluation", evaluation_arg.is_some()),
        ("rederive", rederive),
    ] {
        if present && !allowed.contains(&name) {
            return Err(Failure::usage(format!(
                "--{name} does not apply to --kind {kind}"
            )));
        }
    }
    let report = match kind {
        "profile" => check_profile_bytes(&read_file(file)?)?,
        "observation" => check_observation_bytes(&read_file(file)?)?,
        "subject" => {
            let profile_path = profile_arg
                .ok_or_else(|| Failure::usage("check --kind subject needs --profile"))?;
            check_subject_bytes(&read_file(profile_path)?, &read_file(file)?)?
        }
        "evaluation" => check_evaluation_file(file, rederive, profile_arg, subject_arg)?,
        _ => {
            let profile_path = profile_arg
                .ok_or_else(|| Failure::usage("check --kind assessment needs --profile"))?;
            let evaluation_path = evaluation_arg
                .ok_or_else(|| Failure::usage("check --kind assessment needs --evaluation"))?;
            check_assessment_file(file, evaluation_path, profile_path)?
        }
    };
    emit(out, &to_output_string(&report).map_err(serialise_failure)?)?;
    Ok(if report.valid { EXIT_OK } else { EXIT_INVALID })
}

fn parse_input<T: serde::de::DeserializeOwned>(what: &str, bytes: &[u8]) -> Result<T, Failure> {
    parse_document::<T>(bytes)
        .map_err(|error| Failure::usage(format!("{what} does not parse: {error}")))
}

fn check_evaluation_file(
    file: &str,
    rederive: bool,
    profile: Option<&str>,
    subject: Option<&str>,
) -> Result<CheckReport, Failure> {
    if rederive != (profile.is_some() || subject.is_some())
        || (rederive && (profile.is_none() || subject.is_none()))
    {
        return Err(Failure::usage(
            "--rederive needs --profile and --subject, and they apply only with --rederive",
        ));
    }
    let bytes = read_file(file)?;
    let evaluation = match parse_document::<Evaluation>(&bytes) {
        Ok(evaluation) => evaluation,
        Err(DocError::Schema(message)) => return Ok(schema_report("evaluation", message)),
        Err(DocError::Syntax(message)) => {
            return Err(Failure::usage(format!("evaluation: {message}")));
        }
    };
    let mut report = CheckReport::new("evaluation", check_evaluation(&evaluation));
    if let (true, Some(profile), Some(subject)) = (rederive, profile, subject) {
        // The profile is available, so the structural check also compares classes with it. A
        // profile that does not parse is reported by the rederivation that follows.
        if let Ok(declared) = parse_document::<Profile>(&read_file(profile)?) {
            report
                .errors
                .extend(check_evaluation_classes(&evaluation, &declared));
        }
        let (errors, notes) = rederive_evaluation(&evaluation, profile, subject)?;
        report.errors.extend(errors);
        report.notes = notes;
        report.valid = report.errors.is_empty();
    }
    Ok(report)
}

/// Verify every recorded file digest (exit 3 on a difference), re-run `evaluate` with the
/// recorded `evaluated_at`, and compare.
fn rederive_evaluation(
    recorded: &Evaluation,
    profile_path: &str,
    subject_path: &str,
) -> Result<(Vec<ErrorItem>, Vec<ErrorItem>), Failure> {
    let profile = read_file(profile_path)?;
    let subject = read_file(subject_path)?;
    let plan = read_file(&recorded.run_plan.path)?;
    let battery = read_file(&recorded.battery.path)?;
    let observation = read_file(&recorded.observation.path)?;
    let mut bad = Vec::new();
    for (code, at, bytes, want) in [
        (
            cc::PROFILE_FILE_DIGEST_MISMATCH,
            "profile.sha256",
            &profile,
            &recorded.profile.sha256,
        ),
        (
            cc::SUBJECT_FILE_DIGEST_MISMATCH,
            "subject.sha256",
            &subject,
            &recorded.subject.sha256,
        ),
        (
            cc::RUN_PLAN_FILE_DIGEST_MISMATCH,
            "run_plan.sha256",
            &plan,
            &recorded.run_plan.sha256,
        ),
        (
            cc::BATTERY_FILE_DIGEST_MISMATCH,
            "battery.file_sha256",
            &battery,
            &recorded.battery.file_sha256,
        ),
        (
            cc::OBSERVATION_FILE_DIGEST_MISMATCH,
            "observation.sha256",
            &observation,
            &recorded.observation.sha256,
        ),
    ] {
        if &sha256_hex(bytes) != want {
            bad.push(ErrorItem::new(
                code,
                at,
                "the file's digest differs from the recorded one",
            ));
        }
    }
    if !bad.is_empty() {
        return Err(Failure::binding(
            format!("rederive refused: {} digest mismatch(es)", bad.len()),
            bad,
        ));
    }
    let fresh = evaluate_bytes(
        &profile,
        &subject,
        (&recorded.run_plan.path, &plan),
        (&recorded.battery.path, &battery),
        (&recorded.observation.path, &observation),
        &recorded.evaluated_at,
    )?;
    Ok(compare_evaluations(recorded, &fresh))
}

fn check_assessment_file(
    file: &str,
    evaluation_path: &str,
    profile_path: &str,
) -> Result<CheckReport, Failure> {
    let profile_bytes = read_file(profile_path)?;
    let evaluation_bytes = read_file(evaluation_path)?;
    let profile: Profile = parse_input("--profile", &profile_bytes)?;
    let profile_errors = validate_profile(&profile);
    if !profile_errors.is_empty() {
        return Err(Failure::refused(
            "--profile is not a valid profile; run check --kind profile",
            profile_errors,
        ));
    }
    let evaluation: Evaluation = parse_input("--evaluation", &evaluation_bytes)?;
    let bytes = read_file(file)?;
    let mut report = match parse_document::<Assessment>(&bytes) {
        Ok(assessment) => {
            let mut errors = check_assessment(
                &assessment,
                &AssessmentContext {
                    profile: &profile,
                    profile_bytes: &profile_bytes,
                    evaluation: &evaluation,
                    evaluation_bytes: &evaluation_bytes,
                },
            );
            errors.extend(check_evaluation_citation(
                &assessment.evaluation.path,
                &evaluation_bytes,
            ));
            CheckReport::new("assessment", errors)
        }
        Err(DocError::Schema(message)) => schema_report("assessment", message),
        Err(DocError::Syntax(message)) => {
            return Err(Failure::usage(format!("assessment: {message}")));
        }
    };
    report.purpose = raw_purpose(&bytes);
    Ok(report)
}

/// The path an assessment cites for its evaluation must be readable and hold the bytes that were
/// checked: the pointer is what a later reader follows. A byte-identical copy at the cited path
/// satisfies it; a path that does not resolve, or resolves to different bytes, does not.
fn check_evaluation_citation(cited: &str, supplied: &[u8]) -> Option<ErrorItem> {
    // A path that is not normalised and absolute is already reported by the structural rules, and
    // resolving it would depend on the working directory.
    if !is_normalised_absolute(cited) {
        return None;
    }
    match std::fs::read(cited) {
        Err(error) => Some(ErrorItem::new(
            cc::EVALUATION_PATH_UNRESOLVED,
            "evaluation.path",
            format!("cannot read the cited evaluation: {error}"),
        )),
        Ok(bytes) if bytes != supplied => Some(ErrorItem::new(
            cc::EVALUATION_PATH_MISMATCH,
            "evaluation.path",
            "the cited file's digest differs from the supplied --evaluation",
        )),
        Ok(_) => None,
    }
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
        return Err(Failure::refused(
            "--profile is not a valid profile; run check --kind profile",
            profile_errors,
        ));
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
        PlanError::Refused(errors) => Failure::refused(
            format!("plan refused: {} violated rule(s)", errors.len()),
            errors,
        ),
    })?;
    let text = plan_to_string(&plan).map_err(serialise_failure)?;
    write_output(out, out_path, &text)?;
    Ok(EXIT_OK)
}

/// Write `text` to `--out` (refusing to overwrite) or to stdout.
fn write_output(out: &mut dyn Write, out_path: Option<&str>, text: &str) -> Result<(), Failure> {
    match out_path {
        Some(path) => {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|error| Failure::usage(format!("cannot create {path}: {error}")))?;
            file.write_all(text.as_bytes())
                .map_err(|error| Failure::usage(format!("cannot write {path}: {error}")))
        }
        None => emit(out, text),
    }
}

/// The canonical absolute path of `path`, as a string, and its bytes.
fn read_recorded(path: &str) -> Result<(String, Vec<u8>), Failure> {
    let canonical = std::fs::canonicalize(path)
        .map_err(|error| Failure::usage(format!("cannot resolve {path}: {error}")))?;
    let text = canonical
        .to_str()
        .ok_or_else(|| Failure::usage(format!("{path} is not a UTF-8 path")))?
        .to_string();
    let bytes = read_file(&text)?;
    Ok((text, bytes))
}

fn required<'a>(options: &'a Options, name: &str) -> Result<&'a str, Failure> {
    options
        .one(name)?
        .ok_or_else(|| Failure::usage(format!("evaluate needs --{name}")))
}

fn evaluate(args: &[String], out: &mut dyn Write) -> Result<i32, Failure> {
    let options = Options::parse(
        args,
        &[
            "profile",
            "subject",
            "run-plan",
            "battery",
            "observation",
            "evaluated-at",
            "out",
        ],
    )?;
    if !options.positional.is_empty() {
        return Err(Failure::usage("evaluate takes no positional arguments"));
    }
    let profile = read_file(required(&options, "profile")?)?;
    let subject = read_file(required(&options, "subject")?)?;
    let (plan_path, plan_bytes) = read_recorded(required(&options, "run-plan")?)?;
    let (battery_path, battery_bytes) = read_recorded(required(&options, "battery")?)?;
    let (observation_path, observation_bytes) = read_recorded(required(&options, "observation")?)?;
    let evaluated_at = required(&options, "evaluated-at")?;
    let out_path = options.one("out")?;
    let evaluation = evaluate_bytes(
        &profile,
        &subject,
        (&plan_path, &plan_bytes),
        (&battery_path, &battery_bytes),
        (&observation_path, &observation_bytes),
        evaluated_at,
    )?;
    let text = to_output_string(&evaluation).map_err(serialise_failure)?;
    write_output(out, out_path, &text)?;
    Ok(EXIT_OK)
}

/// Run `evaluate` over file bytes, each with the absolute path to record.
fn evaluate_bytes(
    profile: &[u8],
    subject: &[u8],
    plan: (&str, &[u8]),
    battery: (&str, &[u8]),
    observation: (&str, &[u8]),
    evaluated_at: &str,
) -> Result<Evaluation, Failure> {
    let binary = std::env::current_exe()
        .and_then(std::fs::read)
        .map_err(|error| Failure::usage(format!("cannot read the running binary: {error}")))?;
    let binary_sha256 = sha256_hex(&binary);
    let inputs = EvaluateInputs {
        profile,
        subject,
        run_plan: Recorded {
            path: plan.0,
            bytes: plan.1,
        },
        battery: Recorded {
            path: battery.0,
            bytes: battery.1,
        },
        observation: Recorded {
            path: observation.0,
            bytes: observation.1,
        },
        evaluated_at,
        binary_sha256: &binary_sha256,
    };
    let read = |path: &str| std::fs::read(path);
    run_evaluate(&inputs, &read).map_err(|error| match error {
        EvalError::Input(message) => Failure::usage(message),
        EvalError::Invalid(errors) => Failure::refused(
            format!("evaluate refused: {} violated rule(s)", errors.len()),
            errors,
        ),
        EvalError::Binding(errors) => Failure::binding(
            format!("evaluate refused: {} binding violation(s)", errors.len()),
            errors,
        ),
    })
}

/// `recheck --evaluation E --battery R2 --observation O2`: which derived checks observed
/// something different in a second battery result. Refusals exit 3.
fn recheck(args: &[String], out: &mut dyn Write) -> Result<i32, Failure> {
    let options = Options::parse(args, &["evaluation", "battery", "observation", "out"])?;
    if !options.positional.is_empty() {
        return Err(Failure::usage("recheck takes no positional arguments"));
    }
    let need = |name: &str| -> Result<&str, Failure> {
        options
            .one(name)?
            .ok_or_else(|| Failure::usage(format!("recheck needs --{name}")))
    };
    let evaluation_bytes = read_file(need("evaluation")?)?;
    let second_bytes = read_file(need("battery")?)?;
    let observation_bytes = read_file(need("observation")?)?;
    let out_path = options.one("out")?;
    let refuse = |errors: Vec<ErrorItem>| {
        Failure::binding(
            format!("recheck refused: {} violation(s)", errors.len()),
            errors,
        )
    };
    let evaluation = match parse_document::<Evaluation>(&evaluation_bytes) {
        Ok(evaluation) => evaluation,
        Err(DocError::Schema(message)) => {
            return Err(refuse(vec![ErrorItem::new(
                codes::SCHEMA,
                "evaluation",
                message,
            )]));
        }
        Err(DocError::Syntax(message)) => {
            return Err(Failure::usage(format!("evaluation: {message}")));
        }
    };
    let structural = check_evaluation(&evaluation);
    if !structural.is_empty() {
        return Err(refuse(structural));
    }
    let first_bytes = read_file(&evaluation.battery.path)?;
    let mut errors = Vec::new();
    if sha256_hex(&first_bytes) != evaluation.battery.file_sha256 {
        errors.push(ErrorItem::new(
            cc::BATTERY_FILE_DIGEST_MISMATCH,
            "battery.file_sha256",
            "the original result's digest differs from the evaluation's record",
        ));
    }
    let second: Option<Value> = serde_json::from_slice(&second_bytes).ok();
    if second.is_none() {
        errors.push(ErrorItem::new(bc::RESULT_MALFORMED, "battery", "not JSON"));
    }
    match parse_document::<Observation>(&observation_bytes) {
        Err(DocError::Syntax(message)) => {
            return Err(Failure::usage(format!("observation: {message}")));
        }
        Err(DocError::Schema(message)) => {
            errors.push(ErrorItem::new(
                bc::OBSERVATION_INVALID,
                "observation",
                message,
            ));
        }
        Ok(observation) => {
            for item in validate_observation(&observation) {
                errors.push(ErrorItem::new(
                    bc::OBSERVATION_INVALID,
                    item.at.clone(),
                    format!("{}: {}", item.code, item.message),
                ));
            }
            if observation.result.sha256 != sha256_hex(&second_bytes) {
                errors.push(ErrorItem::new(
                    bc::OBSERVATION_RESULT_DIGEST_MISMATCH,
                    "observation.result.sha256",
                    "differs from the second result's bytes",
                ));
            }
            if observation.plan.sha256 != evaluation.run_plan.sha256 {
                errors.push(ErrorItem::new(
                    bc::OBSERVATION_PLAN_DIGEST_MISMATCH,
                    "observation.plan.sha256",
                    "differs from the evaluation's run plan digest",
                ));
            }
            if let Some(second) = &second {
                let failed = second["failed"].as_array().is_some_and(|f| !f.is_empty());
                if observation.battery_exit != i64::from(failed) {
                    errors.push(ErrorItem::new(
                        bc::BATTERY_EXIT_INCONSISTENT,
                        "observation.battery_exit",
                        "exit 1 exactly when the second result's failed is non-empty",
                    ));
                }
                if second["battery_sha256"].as_str()
                    != Some(observation.stdout_battery_sha256.as_str())
                {
                    errors.push(ErrorItem::new(
                        bc::BATTERY_SHA256_MISMATCH,
                        "observation.stdout_battery_sha256",
                        "differs from the second result's battery_sha256",
                    ));
                }
            }
        }
    }
    let first: Value = serde_json::from_slice(&first_bytes).unwrap_or(Value::Null);
    let (Some(second), true) = (second, errors.is_empty()) else {
        return Err(refuse(errors));
    };
    let changed = changed_checks(&evaluation.derived_check_ids, &first, &second);
    let status = if changed.is_empty() {
        "unchanged"
    } else {
        "observation_changed"
    };
    let text =
        to_output_string(&serde_json::json!({"status": status, "changed_check_ids": changed}))
            .map_err(serialise_failure)?;
    write_output(out, out_path, &text)?;
    Ok(EXIT_OK)
}
