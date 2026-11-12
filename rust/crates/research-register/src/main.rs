//! `work-engine-register snapshot --at <rev> [--register <path>] [--repo <dir>]`
//! `work-engine-register validate --at <rev> [--register <path>] [--repo <dir>]`
//! `work-engine-register anchors --at <rev> [--register <path>] [--repo <dir>]`
//!
//! Each prints one JSON line. Exit codes: 0 ok (snapshot, anchors) or valid (validate), 1 invalid
//! (validate, anchors: the violations are printed as `validate` prints them), 2 usage, 3
//! bad_request, 4 cannot_read, unsupported, no_register_at_commit or (anchors)
//! invariant_failure (the JSON `outcome` names which).

use research_register::{
    AnchorError, Error, GitObjects, Outcome, PinnedSnapshot, SNAPSHOT_SCHEMA, Unreadable,
    Violation, validate_report,
};
use serde::Serialize;
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: work-engine-register (snapshot|validate|anchors) --at <rev> [--register <path>] [--repo <dir>]";
pub const VALIDATION_SCHEMA: &str = "research_register_validation_v1";
pub const ANCHORS_SCHEMA: &str = "research_register_anchors_v1";

#[derive(Serialize)]
struct Failure<'a> {
    schema: &'static str,
    outcome: &'static str,
    requested: &'a str,
    error: String,
    /// A9: violations established before validation stopped at a reading failure. Omitted when
    /// empty, so other failures print as before.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    diagnostics: Vec<ViolationOut<'a>>,
}

fn shown(v: &Violation) -> ViolationOut<'_> {
    ViolationOut {
        rule: v.rule.as_str(),
        file: &v.location.file,
        line: v.location.line,
        pointer: &v.location.pointer,
        detail: &v.detail,
    }
}

#[derive(Serialize)]
struct ViolationOut<'a> {
    rule: &'static str,
    file: &'a str,
    line: Option<u64>,
    pointer: &'a str,
    detail: &'a str,
}

#[derive(Serialize)]
struct Validation<'a> {
    schema: &'static str,
    outcome: &'static str,
    requested: &'a str,
    commit: String,
    register_path: &'a str,
    register_tree: String,
    violations: Vec<ViolationOut<'a>>,
}

#[derive(Clone, Copy, PartialEq)]
enum Command {
    Snapshot,
    Validate,
    Anchors,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((command, at, register, repo)) = parse(&args) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let schema = match command {
        Command::Snapshot => SNAPSHOT_SCHEMA,
        Command::Validate => VALIDATION_SCHEMA,
        Command::Anchors => ANCHORS_SCHEMA,
    };
    let result = GitObjects::open(&repo)
        .and_then(|objects| PinnedSnapshot::build(objects, &at, &register))
        .map_err(|error| Unreadable {
            error,
            diagnostics: Vec::new(),
        })
        .and_then(|snapshot| match command {
            Command::Snapshot => {
                print(&snapshot.identity());
                Ok(ExitCode::SUCCESS)
            }
            Command::Anchors => {
                let outcome = validate_report(&snapshot)?;
                let Outcome::Valid(valid) = &outcome else {
                    let Outcome::Invalid(violations) = &outcome else {
                        unreachable!()
                    };
                    print(&Validation {
                        schema,
                        outcome: "invalid",
                        requested: &at,
                        commit: snapshot.commit().to_string(),
                        register_path: snapshot.register_path(),
                        register_tree: snapshot.register_tree().to_string(),
                        violations: violations.iter().map(shown).collect(),
                    });
                    return Ok(ExitCode::from(1));
                };
                match research_register::anchors(valid, &snapshot) {
                    Ok(report) => {
                        let mut out = report.to_json();
                        let map = out.as_object_mut().expect("report is an object");
                        map.insert("schema".into(), schema.into());
                        map.insert("outcome".into(), "ok".into());
                        map.insert("requested".into(), at.clone().into());
                        print(&out);
                        Ok(ExitCode::SUCCESS)
                    }
                    Err(AnchorError::Read(error)) => Err(Unreadable {
                        error,
                        diagnostics: Vec::new(),
                    }),
                    Err(e @ AnchorError::Invariant(_)) => {
                        print(&Failure {
                            schema,
                            outcome: e.category(),
                            requested: &at,
                            error: e.to_string(),
                            diagnostics: Vec::new(),
                        });
                        Ok(ExitCode::from(4))
                    }
                }
            }
            Command::Validate => {
                let outcome = validate_report(&snapshot)?;
                let (name, code, violations): (_, _, &[Violation]) = match &outcome {
                    Outcome::Valid(_) => ("valid", 0, &[]),
                    Outcome::Invalid(v) => ("invalid", 1, v),
                };
                print(&Validation {
                    schema,
                    outcome: name,
                    requested: &at,
                    commit: snapshot.commit().to_string(),
                    register_path: snapshot.register_path(),
                    register_tree: snapshot.register_tree().to_string(),
                    violations: violations.iter().map(shown).collect(),
                });
                Ok(ExitCode::from(code))
            }
        });
    match result {
        Ok(code) => code,
        Err(Unreadable {
            error: e,
            diagnostics,
        }) => {
            print(&Failure {
                schema,
                outcome: e.category(),
                requested: &at,
                error: e.to_string(),
                diagnostics: diagnostics.iter().map(shown).collect(),
            });
            ExitCode::from(if matches!(e, Error::BadRequest(_)) {
                3
            } else {
                4
            })
        }
    }
}

fn print(value: &impl Serialize) {
    println!(
        "{}",
        serde_json::to_string(value).expect("output serializes")
    );
}

fn parse(args: &[String]) -> Option<(Command, String, String, PathBuf)> {
    let (command, rest) = args.split_first()?;
    let command = match command.as_str() {
        "snapshot" => Command::Snapshot,
        "validate" => Command::Validate,
        "anchors" => Command::Anchors,
        _ => return None,
    };
    let (mut at, mut register, mut repo) =
        (None, "metrics/register".to_string(), PathBuf::from("."));
    let mut it = rest.iter();
    while let Some(flag) = it.next() {
        let value = it.next()?;
        match flag.as_str() {
            "--at" if at.is_none() => at = Some(value.clone()),
            "--register" => register = value.clone(),
            "--repo" => repo = PathBuf::from(value),
            _ => return None,
        }
    }
    Some((command, at?, register, repo))
}
