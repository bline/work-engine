//! Shared fixture support for the integration tests.
//!
//! Fixture trees live under `CARGO_TARGET_TMPDIR`, one directory per test case, rebuilt from
//! scratch on each run. The binary under test is `work-engine-evaluation`.
#![allow(dead_code)]

use proposal_evaluation::sha256_hex;
use proposal_evaluation::validate::{Parsed, parse_estimate_set, validate_estimate_set};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const BIN: &str = env!("CARGO_BIN_EXE_work-engine-evaluation");

pub const HEX_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub const HEX_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

/// A fresh, empty, canonical directory for one test case.
pub fn fresh_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("evaluation-fixtures")
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

/// Write `value` as pretty JSON plus a newline; returns the sha256 of the bytes written.
pub fn write_json(path: &Path, value: &Value) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap();
    text.push('\n');
    write(path, &text);
    sha256_hex(text.as_bytes())
}

pub fn s(path: &Path) -> String {
    path.to_str().unwrap().to_string()
}

/// A basis entry as the fixture projection selects it.
pub fn basis(n: u8) -> Value {
    json!({
        "claim_id": format!("claim-{n}"),
        "claim_revision_id": format!("rev-{n}"),
        "claim_revision_sha256": format!("{n:x}").repeat(64),
    })
}

/// A valid estimate set exercising all three statuses and both scale kinds. Pointer paths and
/// digests are the arguments, so a test can bind real fixture files or dummy ones.
pub fn valid_set(packet_path: &str, packet_sha: &str, proj_path: &str, proj_sha: &str) -> Value {
    json!({
        "schema": "proposal-evaluation-estimate-set-v1",
        "estimate_set_id": "set-1",
        "proposal": {
            "proposal_id": "alpha.beta",
            "packet": {"path": packet_path, "sha256": packet_sha}
        },
        "estimator": {"role": "evaluator", "identity": "tester"},
        "issued_at": "2026-10-22T12:00:00Z",
        "evidence_cutoff": "2026-10-22T10:00:00Z",
        "claim_projection": {"path": proj_path, "sha256": proj_sha},
        "estimates": [
            {
                "status": "estimated",
                "measure_id": "expected-cost",
                "measure": {
                    "definition": "engineering days",
                    "scale": {"kind": "quantity", "unit": "day", "exponent": 0},
                    "direction": "lower_is_better",
                    "scope": "this proposal"
                },
                "evidence_cutoff": "2026-10-21T10:00:00Z",
                "baseline": {"quantity": 10},
                "basis": [basis(1)],
                "limitations": ["synthetic"],
                "value": {"quantity": 8},
                "interval": {"low": {"quantity": 5}, "high": {"quantity": 12}},
                "confidence": "medium"
            },
            {
                "status": "contested",
                "measure_id": "regression_risk",
                "measure": {
                    "definition": "chance of regression",
                    "scale": {"kind": "ordinal", "levels": ["low", "medium", "high"]},
                    "direction": "lower_is_better",
                    "scope": "this proposal"
                },
                "evidence_cutoff": null,
                "baseline": null,
                "basis": [],
                "limitations": [],
                "competing": [
                    {"value": {"level": "low"}, "basis": [basis(2)]},
                    {"value": {"level": "high"}, "basis": [basis(3)]}
                ]
            },
            {
                "status": "not_estimated",
                "measure_id": "unlock.value",
                "measure": {
                    "definition": "unlocked work",
                    "scale": {"kind": "quantity", "unit": "item", "exponent": -1},
                    "direction": "higher_is_better",
                    "scope": "portfolio"
                },
                "evidence_cutoff": null,
                "baseline": null,
                "basis": [],
                "limitations": [],
                "reason": "no evidence yet"
            }
        ],
        "limitations": ["synthetic fixture"],
        "authority": {
            "readiness_judgment_issued": false,
            "priority_authorized": false,
            "acceptance_authorized": false
        }
    })
}

/// The default dummy-pointer set used by structure tests.
pub fn dummy_set() -> Value {
    valid_set("/x/packet.json", HEX_A, "/x/projection.json", HEX_B)
}

/// `(code, at)` pairs from parsing and validating `value` as an estimate set.
pub fn violations(value: &Value) -> Vec<(String, String)> {
    let bytes = serde_json::to_vec(value).unwrap();
    let errors = match parse_estimate_set(&bytes).expect("valid JSON") {
        Parsed::Set(set) => validate_estimate_set(&set),
        Parsed::Invalid(errors) => errors,
    };
    errors.into_iter().map(|e| (e.code, e.at)).collect()
}

pub fn has(found: &[(String, String)], code: &str, at: &str) -> bool {
    found.iter().any(|(c, a)| c == code && a == at)
}

pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    pub fn out_json(&self) -> Value {
        serde_json::from_str(&self.stdout)
            .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {:?}", self.stdout))
    }

    pub fn err_json(&self) -> Value {
        serde_json::from_str(self.stderr.trim())
            .unwrap_or_else(|e| panic!("stderr is not one JSON object ({e}): {:?}", self.stderr))
    }

    /// `(code, at)` pairs from a `check` report (stdout) or a refusal (stderr).
    pub fn pairs(&self) -> Vec<(String, String)> {
        let doc = if self.stdout.is_empty() {
            self.err_json()
        } else {
            self.out_json()
        };
        doc["errors"]
            .as_array()
            .unwrap_or_else(|| panic!("no errors array in {doc}"))
            .iter()
            .map(|e| {
                (
                    e["code"].as_str().unwrap().to_string(),
                    e["at"].as_str().unwrap().to_string(),
                )
            })
            .collect()
    }
}

pub fn cli<S: AsRef<str>>(args: &[S]) -> Run {
    let output = Command::new(BIN)
        .args(args.iter().map(|a| a.as_ref()))
        .output()
        .expect("run work-engine-evaluation");
    Run {
        code: output.status.code().expect("exit code"),
        stdout: String::from_utf8(output.stdout).unwrap(),
        stderr: String::from_utf8(output.stderr).unwrap(),
    }
}
