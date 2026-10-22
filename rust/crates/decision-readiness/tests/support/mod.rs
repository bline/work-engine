//! Shared fixture support for the integration tests.
//!
//! Fixture trees live under `CARGO_TARGET_TMPDIR`, one directory per test case, and are
//! rebuilt from scratch on each run. The binary under test is `work-engine-readiness`.
#![allow(dead_code)]

use decision_readiness::sha256_hex;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub const BIN: &str = env!("CARGO_BIN_EXE_work-engine-readiness");

/// The unmodified evidence battery in the repository this crate lives in.
pub fn battery_script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../skills/evidence-battery/scripts/evidence_battery.py")
}

pub fn c1_profile_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/integration-readiness-v1.json")
}

/// A fresh, empty, canonical directory for one test case.
pub fn fresh_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("readiness-fixtures")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::canonicalize(&dir).unwrap()
}

pub fn write(path: &Path, text: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, text).unwrap();
}

pub fn write_json(path: &Path, value: &Value) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap();
    text.push('\n');
    write(path, &text);
    sha256_hex(text.as_bytes())
}

pub fn file_sha(path: &Path) -> String {
    sha256_hex(&std::fs::read(path).unwrap())
}

pub fn s(path: &Path) -> String {
    path.to_str().unwrap().to_string()
}

pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    /// The JSON on stdout (a `check` report or a plan).
    pub fn out_json(&self) -> Value {
        serde_json::from_str(&self.stdout)
            .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {:?}", self.stdout))
    }

    /// The one JSON object a refusal writes to stderr.
    pub fn err_json(&self) -> Value {
        serde_json::from_str(self.stderr.trim())
            .unwrap_or_else(|e| panic!("stderr is not one JSON object ({e}): {:?}", self.stderr))
    }

    /// Error codes, from a `check` report (stdout) or a refusal (stderr).
    pub fn codes(&self) -> Vec<String> {
        let doc = if self.stdout.is_empty() {
            self.err_json()
        } else {
            self.out_json()
        };
        doc["errors"]
            .as_array()
            .unwrap_or_else(|| panic!("no errors array in {doc}"))
            .iter()
            .map(|e| e["code"].as_str().unwrap().to_string())
            .collect()
    }
}

pub fn cli<S: AsRef<str>>(args: &[S]) -> Run {
    let output: Output = Command::new(BIN)
        .args(args.iter().map(|a| a.as_ref()))
        .output()
        .expect("run work-engine-readiness");
    Run {
        code: output.status.code().expect("exit code"),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}

/// Run git in `dir` with a fixed identity and no user or system configuration.
pub fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
        ])
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .trim_end()
        .to_string()
}

/// A manifest file over `files` (paths relative to `root`), written at `manifest_path`.
/// Returns the manifest value `{root, path, sha256}` for a subject, with `manifest_root` naming
/// the root the manifest file itself sits under.
pub fn manifest(
    manifest_dir: &Path,
    manifest_rel: &str,
    manifest_root: &str,
    entries_dir: &Path,
    files: &[&str],
) -> Value {
    let entries: Vec<Value> = files
        .iter()
        .map(|rel| json!({"path": rel, "sha256": file_sha(&entries_dir.join(rel))}))
        .collect();
    let sha = write_json(&manifest_dir.join(manifest_rel), &Value::Array(entries));
    json!({"root": manifest_root, "path": manifest_rel, "sha256": sha})
}

/// JSON text of `value` with every object's keys in reverse order: a permuted-key equivalent.
pub fn permuted_text(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let members: Vec<String> = map
                .iter()
                .rev()
                .map(|(k, v)| format!("{}:{}", serde_json::to_string(k).unwrap(), permuted_text(v)))
                .collect();
            format!("{{{}}}", members.join(","))
        }
        Value::Array(items) => {
            let parts: Vec<String> = items.iter().map(permuted_text).collect();
            format!("[{}]", parts.join(","))
        }
        other => serde_json::to_string(other).unwrap(),
    }
}

/// A complete fixture: a profile exercising all eight predicate kinds, a subject, and the
/// tree it names. `profile` and `subject` are editable before `write_documents`.
pub struct Case {
    pub dir: PathBuf,
    pub dest: PathBuf,
    pub cand: PathBuf,
    pub evid: PathBuf,
    pub bins: PathBuf,
    pub profile: Value,
    pub subject: Value,
}

impl Case {
    pub fn profile_path(&self) -> PathBuf {
        self.dir.join("profile.json")
    }
    pub fn subject_path(&self) -> PathBuf {
        self.dir.join("subject.json")
    }

    /// Write `profile` and `subject` (with the profile's digest filled in) and return the digest.
    pub fn write_documents(&mut self) -> String {
        let sha = write_json(&self.profile_path(), &self.profile);
        self.subject["profile"]["sha256"] = json!(sha);
        write_json(&self.subject_path(), &self.subject);
        sha
    }

    pub fn check_profile(&self) -> Run {
        cli(&["check", "--kind", "profile", &s(&self.profile_path())])
    }
    pub fn check_subject(&self) -> Run {
        cli(&[
            "check",
            "--kind",
            "subject",
            "--profile",
            &s(&self.profile_path()),
            &s(&self.subject_path()),
        ])
    }
    pub fn plan(&self) -> Run {
        cli(&[
            "plan",
            "--profile",
            &s(&self.profile_path()),
            "--subject",
            &s(&self.subject_path()),
        ])
    }
}

/// The base fixture, written to disk and valid under both `check` and `plan`.
pub fn base_case(name: &str) -> Case {
    let dir = fresh_dir(name);
    let (dest, cand, evid, bins) = (
        dir.join("dest"),
        dir.join("cand"),
        dir.join("evid"),
        dir.join("bins"),
    );
    for d in [&dest, &cand, &evid, &bins] {
        std::fs::create_dir_all(d).unwrap();
    }
    // Destination: a git repository with one commit.
    git(&dest, &["init", "--quiet"]);
    write(&dest.join("tracked.txt"), "tracked\n");
    write(&dest.join("other.txt"), "other\n");
    git(&dest, &["add", "tracked.txt", "other.txt"]);
    git(&dest, &["commit", "--quiet", "-m", "baseline"]);
    let head = git(&dest, &["rev-parse", "HEAD"]);
    // Candidate, binaries, evidence.
    write(&cand.join("src/a.txt"), "alpha\n");
    write(&cand.join("src/b.txt"), "beta\n");
    write(&bins.join("bin/tool"), "tool-bytes\n");
    write_json(
        &evid.join("validation.json"),
        &json!({
            "status": "ok",
            "count": 3,
            "ratio": 0.5,
            "flag": true,
            "tags": ["x", "y"],
            "detail": {"a": 1, "b": [true, "x"]},
            "review": {"conclusion": "sufficient"},
            "suites": {"core": {"exit_code": 0, "counts": {"failed": 0}}, "ui": {"exit_code": 0, "counts": {"failed": 0}}}
        }),
    );
    write(
        &evid.join("logs/core.log"),
        "running 3 tests\ntest result: ok. 3 passed; 0 failed\n",
    );
    write(
        &evid.join("logs/ui.log"),
        "running 1 test\ntest result: ok. 1 passed; 0 failed\n",
    );
    let source_manifest = manifest(
        &evid,
        "source-manifest.json",
        "evid",
        &cand,
        &["src/a.txt", "src/b.txt"],
    );
    let evidence_manifest = manifest(
        &evid,
        "evidence-manifest.json",
        "evid",
        &evid,
        &["validation.json", "logs/core.log", "logs/ui.log"],
    );
    let a_sha = file_sha(&cand.join("src/a.txt"));

    let profile = json!({
        "schema": "decision-readiness-profile-v1",
        "profile_id": "fixture.base",
        "version": 1,
        "decision": {"id": "fixture-decision", "owner_role": "coordinator"},
        "profile_owner": "coordinator",
        "assessor_roles": ["reviewer"],
        "authority_refs": [{"path": "notes/fixture.md", "lines": "1-2"}],
        "roots": [{"name": "dest", "git": true}, {"name": "cand"}, {"name": "evid"}, {"name": "bins"}],
        "parameters": [
            {"name": "source_manifest", "kind": "manifest", "entries_root": "cand"},
            {"name": "evidence_manifest", "kind": "manifest", "entries_root": "evid"},
            {"name": "validation_record", "kind": "path", "bound_by": "evidence_manifest"},
            {"name": "logs", "kind": "path_list", "bound_by": "evidence_manifest"},
            {"name": "suites", "kind": "list"},
            {"name": "head", "kind": "string"},
            {"name": "affected", "kind": "list"}
        ],
        "requirements": [
            {"id": "R-file", "class": "mechanical", "statement": "a.txt has its recorded bytes",
             "predicates": [{"kind": "file_sha256", "file": {"root": "cand", "path": "src/a.txt"}, "expected": a_sha}]},
            {"id": "R-manifest", "class": "mechanical", "statement": "candidate matches manifest",
             "predicates": [{"kind": "manifest_match", "manifest": {"param": "source_manifest"}}]},
            {"id": "R-json", "class": "mechanical", "statement": "json_equals over several types",
             "predicates": [
                {"kind": "json_equals", "file": {"param": "validation_record"}, "key": "status", "expected": "ok"},
                {"kind": "json_equals", "file": {"param": "validation_record"}, "key": "count", "expected": 3},
                {"kind": "json_equals", "file": {"param": "validation_record"}, "key": "flag", "expected": true},
                {"kind": "json_equals", "file": {"param": "validation_record"}, "key": "tags", "expected": ["x", "y"]},
                {"kind": "json_equals", "file": {"param": "validation_record"}, "key": "detail", "expected": {"a": 1, "b": [true, "x"]}},
                {"kind": "json_equals", "file": {"param": "validation_record"}, "key": "status", "expected": {"param": "head"}}
             ]},
            {"id": "R-compare", "class": "mechanical", "statement": "json_compare with each",
             "predicates": [
                {"kind": "json_compare", "file": {"param": "validation_record"}, "key": "suites.{each}.exit_code", "each": {"param": "suites"}, "op": "eq", "value": 0},
                {"kind": "json_compare", "file": {"param": "validation_record"}, "key": "ratio", "op": "le", "value": 0.75},
                {"kind": "json_compare", "file": {"param": "validation_record"}, "key": "count", "op": "ge", "value": 1}
             ]},
            {"id": "R-present", "class": "mechanical", "statement": "json_present",
             "predicates": [
                {"kind": "json_present", "file": {"param": "validation_record"}, "key": "review.conclusion"},
                {"kind": "json_present", "file": {"root": "evid", "path": "validation.json"}, "key": "suites.{each}.counts.failed", "each": {"param": "suites"}}
             ]},
            {"id": "R-grep", "class": "mechanical", "statement": "grep whole-set and per-path",
             "predicates": [
                {"kind": "grep_count", "pattern": "test result: FAILED", "files": {"param": "logs"}, "max": 0},
                {"kind": "grep_count", "pattern": "test result: ok", "files": {"param": "logs"}, "min": 1, "per_path": true},
                {"kind": "grep_count", "pattern": "running", "files": [{"root": "evid", "path": "logs/core.log"}, {"param": "validation_record"}], "min": 1}
             ]},
            {"id": "R-git", "class": "mechanical", "statement": "destination baseline",
             "predicates": [
                {"kind": "git_output_equals", "args": ["rev-parse", "HEAD"], "expected": {"param": "head"}},
                {"kind": "git_output_empty", "args": ["status", "--porcelain", "--", {"param": "affected"}]}
             ]},
            {"id": "A-note", "class": "attributed", "statement": "a judgment",
             "evidence_from": ["R-file", "R-git"]}
        ]
    });
    let subject = json!({
        "schema": "decision-readiness-subject-v1",
        "profile": {"id": "fixture.base", "version": 1, "sha256": "0".repeat(64)},
        "subject_id": "fixture-subject",
        "author": "tests",
        "roots": {"dest": s(&dest), "cand": s(&cand), "evid": s(&evid), "bins": s(&bins)},
        "values": {
            "source_manifest": source_manifest,
            "evidence_manifest": evidence_manifest,
            "validation_record": {"root": "evid", "path": "validation.json"},
            "logs": [{"root": "evid", "path": "logs/core.log"}, {"root": "evid", "path": "logs/ui.log"}],
            "suites": ["core", "ui"],
            "head": head,
            "affected": ["tracked.txt", "other.txt"]
        }
    });
    let mut case = Case {
        dir,
        dest,
        cand,
        evid,
        bins,
        profile,
        subject,
    };
    case.write_documents();
    case
}

/// Run `evidence_battery.py --plan P --save R` and return `(exit code, result JSON)`.
pub fn run_battery(plan: &Path, result: &Path) -> (i32, Value) {
    let output = Command::new("python3")
        .arg("-I")
        .arg(battery_script())
        .arg("--plan")
        .arg(plan)
        .arg("--save")
        .arg(result)
        .arg("--quiet")
        .output()
        .expect("run python3");
    let code = output.status.code().expect("battery exit code");
    assert!(
        code == 0 || code == 1,
        "battery exit {code}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let saved: Value = serde_json::from_slice(&std::fs::read(result).unwrap()).unwrap();
    (code, saved)
}
