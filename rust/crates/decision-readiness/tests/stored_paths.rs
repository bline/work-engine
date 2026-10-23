//! Paths read from stored documents (a recorded input of an evaluation, a subject's manifest)
//! must not stall the tool: a FIFO has no writer, so opening it waits indefinitely. Each guarded
//! read site is run here with a FIFO, a symlink to one, and a directory at the stored path, under
//! an external deadline so a regression fails the test instead of hanging the run. The citation
//! site is covered in `assessment.rs`.
//!
//! The pathname pre-check alone refuses every static case below, so these tests pin that each
//! site goes through the guarded reader at all; the descriptor check and `O_NONBLOCK`, which hold
//! when a path is swapped after the pre-check, are pinned by the unit test of `open_regular` in
//! `src/cli.rs`.

mod support;

use serde_json::json;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use support::*;

const DEADLINE: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, Debug)]
enum Special {
    Fifo,
    FifoLink,
    Directory,
}

const SPECIALS: [Special; 3] = [Special::Fifo, Special::FifoLink, Special::Directory];

/// Create a special object of `kind` at `at`, which must not exist. A link points at a FIFO
/// created beside it.
fn make_special(kind: Special, at: &Path) {
    match kind {
        Special::Fifo => make_fifo(at),
        Special::FifoLink => {
            let target = at.with_extension("target.fifo");
            make_fifo(&target);
            std::os::unix::fs::symlink(&target, at).unwrap();
        }
        Special::Directory => std::fs::create_dir(at).unwrap(),
    }
}

fn run_deadline(tag: &str, args: &[String]) -> Run {
    cli_with_deadline(args, DEADLINE)
        .unwrap_or_else(|| panic!("{tag}: no exit within {DEADLINE:?}; a stored path blocked"))
}

/// A stored path that is not a regular file is refused as an unreadable input (exit 2), naming it.
fn assert_not_regular(tag: &str, run: &Run, path: &Path) {
    assert_eq!(run.code, 2, "{tag}: {} {}", run.stdout, run.stderr);
    assert!(run.stdout.is_empty(), "{tag}: {}", run.stdout);
    let message = run.err_json()["error"].as_str().unwrap().to_string();
    assert!(message.contains("not a regular file"), "{tag}: {message}");
    assert!(message.contains(&s(path)), "{tag}: {message}");
}

struct Recorded {
    case: Case,
    observed: Observed,
    evaluation: PathBuf,
}

fn recorded(name: &str) -> Recorded {
    let case = met_case(name);
    let observed = observe(&case, "run");
    let evaluation = evaluation_file(&case, &observed, "e.json");
    Recorded {
        case,
        observed,
        evaluation,
    }
}

fn rederive_args(r: &Recorded, evaluation: &Path) -> Vec<String> {
    [
        "check",
        "--kind",
        "evaluation",
        &s(evaluation),
        "--rederive",
        "--profile",
        &s(&r.case.profile_path()),
        "--subject",
        &s(&r.case.subject_path()),
    ]
    .map(String::from)
    .to_vec()
}

/// `--rederive` re-reads the three inputs an evaluation records by path.
fn rederive_site(field: &str) {
    let r = recorded(&format!("stored-rederive-{field}"));
    let control = run_deadline("control", &rederive_args(&r, &r.evaluation));
    assert_eq!(control.code, 0, "control: {}", control.stdout);
    for kind in SPECIALS {
        let tag = format!("{field}-{kind:?}");
        let special = r.case.dir.join(format!("{tag}.special"));
        make_special(kind, &special);
        let mut value = read_json(&r.evaluation);
        value[field]["path"] = json!(s(&special));
        let doc = r.case.dir.join(format!("{tag}.evaluation.json"));
        write_json(&doc, &value);
        let run = run_deadline(&tag, &rederive_args(&r, &doc));
        assert_not_regular(&tag, &run, &special);
    }
}

#[test]
fn rederive_run_plan_path_that_is_not_a_plain_file_is_refused_without_blocking() {
    rederive_site("run_plan");
}

#[test]
fn rederive_battery_path_that_is_not_a_plain_file_is_refused_without_blocking() {
    rederive_site("battery");
}

#[test]
fn rederive_observation_path_that_is_not_a_plain_file_is_refused_without_blocking() {
    rederive_site("observation");
}

#[test]
fn recheck_first_battery_path_that_is_not_a_plain_file_is_refused_without_blocking() {
    let r = recorded("stored-recheck");
    let second = observe(&r.case, "run2");
    let args = |evaluation: &Path| -> Vec<String> {
        [
            "recheck",
            "--evaluation",
            &s(evaluation),
            "--battery",
            &s(&second.result),
            "--observation",
            &s(&second.observation),
        ]
        .map(String::from)
        .to_vec()
    };
    let control = run_deadline("control", &args(&r.evaluation));
    assert_eq!(control.code, 0, "control: {}", control.stderr);
    for kind in SPECIALS {
        let tag = format!("battery-{kind:?}");
        let special = r.case.dir.join(format!("{tag}.special"));
        make_special(kind, &special);
        let mut value = read_json(&r.evaluation);
        value["battery"]["path"] = json!(s(&special));
        let doc = r.case.dir.join(format!("{tag}.evaluation.json"));
        write_json(&doc, &value);
        let run = run_deadline(&tag, &args(&doc));
        assert_not_regular(&tag, &run, &special);
    }
}

/// Replace the evidence manifest a subject names (`evid/evidence-manifest.json`) with a special
/// object, and run `command` over the case.
fn manifest_site(name: &str, command: impl Fn(&Recorded) -> Vec<String>) {
    let r = recorded(name);
    let control = run_deadline("control", &command(&r));
    assert_eq!(
        control.code, 0,
        "control: {} {}",
        control.stdout, control.stderr
    );
    let manifest = r.case.evid.join("evidence-manifest.json");
    for kind in SPECIALS {
        std::fs::remove_file(&manifest).ok();
        std::fs::remove_dir_all(&manifest).ok();
        std::fs::remove_file(manifest.with_extension("target.fifo")).ok();
        make_special(kind, &manifest);
        let tag = format!("manifest-{kind:?}");
        let run = run_deadline(&tag, &command(&r));
        assert_not_regular(&tag, &run, &manifest);
    }
}

#[test]
fn check_subject_manifest_path_that_is_not_a_plain_file_is_refused_without_blocking() {
    manifest_site("stored-manifest-check", |r| {
        [
            "check",
            "--kind",
            "subject",
            "--profile",
            &s(&r.case.profile_path()),
            &s(&r.case.subject_path()),
        ]
        .map(String::from)
        .to_vec()
    });
}

#[test]
fn plan_manifest_path_that_is_not_a_plain_file_is_refused_without_blocking() {
    manifest_site("stored-manifest-plan", |r| {
        [
            "plan",
            "--profile",
            &s(&r.case.profile_path()),
            "--subject",
            &s(&r.case.subject_path()),
        ]
        .map(String::from)
        .to_vec()
    });
}

#[test]
fn evaluate_manifest_path_that_is_not_a_plain_file_is_refused_without_blocking() {
    manifest_site("stored-manifest-evaluate", |r| {
        [
            "evaluate",
            "--profile",
            &s(&r.case.profile_path()),
            "--subject",
            &s(&r.case.subject_path()),
            "--run-plan",
            &s(&r.observed.plan),
            "--battery",
            &s(&r.observed.result),
            "--observation",
            &s(&r.observed.observation),
            "--evaluated-at",
            EVALUATED_AT,
        ]
        .map(String::from)
        .to_vec()
    });
}

/// The deadline helper must not mistake a chatty child for a blocked one: output well past a pipe
/// buffer (64 KiB on Linux) on both streams is collected, and a sleeping child is still killed.
#[test]
fn the_deadline_helper_drains_large_output_and_still_kills_a_hang() {
    let mut chatty = Command::new("sh");
    chatty.arg("-c").arg(
        "head -c 1000000 /dev/zero | tr '\\0' a; head -c 1000000 /dev/zero | tr '\\0' b >&2; exit 7",
    );
    let run = run_with_deadline(chatty, Duration::from_secs(10)).expect("chatty child exits");
    assert_eq!(run.code, 7);
    assert_eq!(run.stdout.len(), 1_000_000);
    assert!(run.stdout.bytes().all(|b| b == b'a'));
    assert_eq!(run.stderr.len(), 1_000_000);

    let mut sleeper = Command::new("sleep");
    sleeper.arg("30");
    let started = std::time::Instant::now();
    assert!(run_with_deadline(sleeper, Duration::from_millis(300)).is_none());
    assert!(started.elapsed() < Duration::from_secs(10));
}
