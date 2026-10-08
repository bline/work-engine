//! Offline component command surface. It has no provider or domain execution dependency.

use std::fs;
use std::path::{Path, PathBuf};

use lifecycle_admin::recovery::{apply_qualified_fact, inspect_exact, retire_exact_no_retry};
use lifecycle_admin::selection::{FileLaunchSelectionPort, LaunchSelectionPort, SelectionRecord};
use lifecycle_admin::transfer::{
    TransferCut, resume_transfer_with_barrier, transfer_current_with_barrier,
};
use lifecycle_core::{HandoffId, SubjectId};
use lifecycle_store::{
    ExactAttemptSelector, QualifiedRecoveryFact, SignedDispositionRequest, SqliteLifecycleStore,
};
use serde::Serialize;
use serde::de::DeserializeOwned;

const FIXTURE_ISSUER: &str = "test-issuer";

fn main() {
    if let Err(error) = run(std::env::args().skip(1).collect()) {
        eprintln!("lifecycle-admin: {error}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    let command = args.first().map(String::as_str).ok_or_else(usage)?;
    match (command, args.len()) {
        ("inspect", 3) => {
            let store = open(&args[1])?;
            let selector: ExactAttemptSelector = json_file(&args[2])?;
            emit(&inspect_exact(&store, &selector).map_err(display)?)
        }
        ("apply-fact", 3) => {
            let mut store = open(&args[1])?;
            let fact: QualifiedRecoveryFact = json_file(&args[2])?;
            emit(&apply_qualified_fact(&mut store, &fact).map_err(display)?)
        }
        ("retire-no-retry", 3) => {
            let mut store = open(&args[1])?;
            let request: SignedDispositionRequest = json_file(&args[2])?;
            emit(&retire_exact_no_retry(&mut store, &request).map_err(display)?)
        }
        ("export-current", 3) => {
            let store = open(&args[1])?;
            emit(&store.export_current(Path::new(&args[2])).map_err(display)?)
        }
        ("transfer-current" | "resume-transfer", 7) => {
            let mut source = open(&args[1])?;
            let mut selection =
                FileLaunchSelectionPort::open(Path::new(&args[2])).map_err(display)?;
            let expected: SelectionRecord = json_file(&args[3])?;
            let handoff = HandoffId::parse(args[6].clone()).map_err(display)?;
            let capsule = PathBuf::from(&args[4]);
            let destination = PathBuf::from(&args[5]);
            let receipt = if command == "transfer-current" {
                transfer_current_with_barrier(
                    &mut source,
                    &mut selection,
                    &expected,
                    &capsule,
                    &destination,
                    &handoff,
                    FIXTURE_ISSUER.to_owned(),
                    &mut component_cut,
                )
            } else {
                resume_transfer_with_barrier(
                    &mut source,
                    &mut selection,
                    &expected,
                    &capsule,
                    &destination,
                    &handoff,
                    FIXTURE_ISSUER.to_owned(),
                    &mut component_cut,
                )
            }
            .map_err(display)?;
            emit(&receipt)
        }
        ("hold-selection", 3) if cfg!(debug_assertions) => {
            let mut selection =
                FileLaunchSelectionPort::open(Path::new(&args[1])).map_err(display)?;
            let _guard = selection.lock().map_err(display)?;
            write_barrier(Path::new(&args[2]), b"selection_held")?;
            use std::io::Read;
            let mut input = Vec::new();
            std::io::stdin().read_to_end(&mut input).map_err(display)?;
            Ok(())
        }
        ("publish-artifact-cut", 5) if cfg!(debug_assertions) => {
            let mut store = open(&args[1])?;
            let subject = SubjectId::parse(args[2].clone()).map_err(display)?;
            let bytes = fs::read(&args[3]).map_err(display)?;
            let staged = store.stage_artifact(subject, &bytes).map_err(display)?;
            store
                .publish_artifact_with_barrier(
                    staged,
                    lifecycle_core::ClockSample {
                        wall: lifecycle_core::WallTimeMs::new(200),
                        wait_budget: lifecycle_core::WaitBudgetMs::new(100),
                    },
                    Some(Path::new(&args[4])),
                )
                .map_err(display)?;
            Ok(())
        }
        _ => Err(usage()),
    }
}

fn component_cut(cut: TransferCut) {
    if !cfg!(debug_assertions) {
        return;
    }
    let Ok(expected) = std::env::var("LIFECYCLE_ADMIN_TEST_CUT") else {
        return;
    };
    if expected != cut.as_str() {
        return;
    }
    let path =
        std::env::var("LIFECYCLE_ADMIN_TEST_BARRIER").expect("component cut requires barrier path");
    write_barrier(Path::new(&path), cut.as_str().as_bytes())
        .expect("component barrier must be durable");
    std::process::exit(86);
}

fn write_barrier(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::fs::{File, OpenOptions};
    use std::io::Write;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(display)?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(display)?;
    let parent = path
        .parent()
        .ok_or_else(|| "barrier has no parent".to_owned())?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(display)?;
    Ok(())
}

fn open(root: &str) -> Result<SqliteLifecycleStore, String> {
    SqliteLifecycleStore::open(Path::new(root), FIXTURE_ISSUER.to_owned()).map_err(display)
}

fn json_file<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let bytes = fs::read(path).map_err(display)?;
    serde_json::from_slice(&bytes).map_err(display)
}

fn emit<T: Serialize>(value: &T) -> Result<(), String> {
    println!("{}", serde_json::to_string_pretty(value).map_err(display)?);
    Ok(())
}

fn display(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn usage() -> String {
    "usage: lifecycle-admin inspect|apply-fact|retire-no-retry <store-root> <request.json>; export-current <store-root> <absent-capsule-root>; transfer-current|resume-transfer <source-root> <selection-directory> <expected-selection.json> <capsule-root> <destination-root> <handoff-id>".to_owned()
}
