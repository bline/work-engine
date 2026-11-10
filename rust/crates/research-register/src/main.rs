//! `work-engine-register snapshot --at <rev> [--register <path>] [--repo <dir>]`
//!
//! Prints one JSON line. Exit codes: 0 ok, 2 usage, 3 bad_request, 4 cannot_read, unsupported or
//! no_register_at_commit (the JSON `outcome` names which).

use research_register::{Error, GitObjects, PinnedSnapshot, SNAPSHOT_SCHEMA};
use serde::Serialize;
use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: work-engine-register snapshot --at <rev> [--register <path>] [--repo <dir>]";

#[derive(Serialize)]
struct Failure<'a> {
    schema: &'static str,
    outcome: &'static str,
    requested: &'a str,
    error: String,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((at, register, repo)) = parse(&args) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    let result = GitObjects::open(&repo).and_then(|objects| PinnedSnapshot::build(objects, &at, &register));
    match result {
        Ok(snapshot) => {
            println!("{}", serde_json::to_string(&snapshot.identity()).expect("identity serializes"));
            ExitCode::SUCCESS
        }
        Err(e) => {
            let failure = Failure { schema: SNAPSHOT_SCHEMA, outcome: e.category(), requested: &at, error: e.to_string() };
            println!("{}", serde_json::to_string(&failure).expect("failure serializes"));
            ExitCode::from(if matches!(e, Error::BadRequest(_)) { 3 } else { 4 })
        }
    }
}

fn parse(args: &[String]) -> Option<(String, String, PathBuf)> {
    let (command, rest) = args.split_first()?;
    if command != "snapshot" {
        return None;
    }
    let (mut at, mut register, mut repo) = (None, "metrics/register".to_string(), PathBuf::from("."));
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
    Some((at?, register, repo))
}
