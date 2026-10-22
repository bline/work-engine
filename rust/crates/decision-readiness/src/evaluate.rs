//! `evaluate`: bind a battery result to a derived plan and compute mechanical statuses
//! (contract 5.2, 5.3, 3.6).
//!
//! `evaluate` is a pure function of its input bytes, the recorded input paths and the tool
//! binary's digest. It reads no clock and observes nothing: the evidence battery observed.
//! It writes a status per predicate and per requirement. It never writes a verdict;
//! [`Evaluation`] has no verdict field.
//!
//! Interpretations settled for this build (each is stated in the slice result):
//! - Invalid profile or subject inputs, including a manifest digest mismatch, are exit 2 as
//!   in `plan` (errata E6); only the 5.2 and 5.3 binding refusals are exit 3.
//! - `manifest_changed` is checked before `digest_differs` (errata E5).
//! - A result entry whose fields are missing or mistyped is `result_malformed`, not
//!   `echo_mismatch`, so status code never meets a shape it did not validate.
//! - An observation record is bound to the files by digest only; its recorded paths are not
//!   compared with the paths passed to `evaluate`.
//! - A check that carries `error` is recorded in `binding.checks` as `none`: nothing of its
//!   echo was verified.

use crate::diag::ErrorItem;
use crate::digest::{is_sha256_hex, sha256_hex};
use crate::model::*;
use crate::plan::{
    CheckSpec, Derivation, PairBytes, PlanCheck, PlanError, PredicateChecks, derive, load_pairs,
};
use crate::validate::{
    DocError, ReadFile, TypedValue, is_utc_timestamp, parse_document, validate_observation,
};
use serde_json::{Map, Number, Value};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

pub const TOOL_NAME: &str = "work-engine-readiness";

/// Closed set of predicate reasons (contract 3.6).
pub const PREDICATE_REASONS: [&str; 17] = [
    "git_root_mismatch",
    "existence_check_error",
    "absent",
    "directory",
    "symlink",
    "check_error",
    "observation_inconsistent",
    "manifest_changed",
    "digest_differs",
    "differs",
    "out_of_bound",
    "type_mismatch",
    "null_or_missing",
    "count_out_of_bound",
    "git_nonzero",
    "output_truncated",
    "not_empty",
];

pub const ATTRIBUTED_REASON: &str = "attributed_judgment_required";

/// Binding refusal codes (contract 5.2).
pub mod binding_codes {
    pub const RUN_PLAN_MALFORMED: &str = "run_plan_malformed";
    pub const RUN_PLAN_DUPLICATE_ID: &str = "run_plan_duplicate_id";
    pub const DERIVED_CHECK_MISSING: &str = "derived_check_missing";
    pub const DERIVED_CHECK_ALTERED: &str = "derived_check_altered";
    pub const RUN_PLAN_ROOT_MISMATCH: &str = "run_plan_root_mismatch";
    pub const RUN_PLAN_ROOT_MISSING: &str = "run_plan_root_missing";
    pub const OBSERVATION_INVALID: &str = "observation_invalid";
    pub const OBSERVATION_PLAN_DIGEST_MISMATCH: &str = "observation_plan_digest_mismatch";
    pub const OBSERVATION_RESULT_DIGEST_MISMATCH: &str = "observation_result_digest_mismatch";
    pub const BATTERY_EXIT_INCONSISTENT: &str = "battery_exit_inconsistent";
    pub const BATTERY_SHA256_MISMATCH: &str = "battery_sha256_mismatch";
    pub const RESULT_MALFORMED: &str = "result_malformed";
    pub const RESULT_ENTRY_MISSING: &str = "result_entry_missing";
    pub const RESULT_KIND_MISMATCH: &str = "result_kind_mismatch";
    pub const ECHO_MISMATCH: &str = "echo_mismatch";
}
use binding_codes as bc;

/// A file given to `evaluate`: the canonical absolute path to record, and its bytes.
#[derive(Debug, Clone, Copy)]
pub struct Recorded<'a> {
    pub path: &'a str,
    pub bytes: &'a [u8],
}

pub struct EvaluateInputs<'a> {
    pub profile: &'a [u8],
    pub subject: &'a [u8],
    pub run_plan: Recorded<'a>,
    pub battery: Recorded<'a>,
    pub observation: Recorded<'a>,
    pub evaluated_at: &'a str,
    pub binary_sha256: &'a str,
}

/// Why `evaluate` produced no evaluation.
#[derive(Debug)]
pub enum EvalError {
    /// A usage, parse or IO failure: exit 2.
    Input(String),
    /// A profile or subject that violates a rule: exit 2, as in `plan`.
    Invalid(Vec<ErrorItem>),
    /// A binding refusal, every violation listed: exit 3.
    Binding(Vec<ErrorItem>),
}

fn binding_error(code: &str, at: impl Into<String>, message: impl Into<String>) -> ErrorItem {
    ErrorItem::new(code, at, message)
}

// ---------------------------------------------------------------------------
// What the battery observed, parsed once and typed
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryType {
    Absent,
    File,
    Symlink,
    Directory,
}

#[derive(Debug, Clone)]
struct ShaEntry {
    path: String,
    kind: EntryType,
    sha256: Option<String>,
}

#[derive(Debug, Clone)]
enum Observed {
    Error(String),
    Sha {
        path_count: u64,
        entries_sha256: String,
        entries: Vec<ShaEntry>,
    },
    Json {
        sha256: String,
        selected: Map<String, Value>,
    },
    Grep {
        match_count: u64,
        match_paths: Vec<String>,
    },
    Git {
        returncode: i64,
        output_sha256: String,
        byte_count: u64,
        text: String,
        truncated: bool,
        stderr: Option<String>,
    },
}

fn spec_kind(spec: &CheckSpec) -> &'static str {
    match spec {
        CheckSpec::Sha256(_) => "sha256",
        CheckSpec::Json(_) => "json",
        CheckSpec::Grep(_) => "grep",
        CheckSpec::Git(_) => "git",
    }
}

fn field<'a>(object: &'a Map<String, Value>, name: &str) -> Result<&'a Value, String> {
    object.get(name).ok_or_else(|| format!("{name} is missing"))
}

fn field_u64(object: &Map<String, Value>, name: &str) -> Result<u64, String> {
    field(object, name)?
        .as_u64()
        .ok_or_else(|| format!("{name} is not a non-negative integer"))
}

fn field_str<'a>(object: &'a Map<String, Value>, name: &str) -> Result<&'a str, String> {
    field(object, name)?
        .as_str()
        .ok_or_else(|| format!("{name} is not a string"))
}

fn field_digest(object: &Map<String, Value>, name: &str) -> Result<String, String> {
    let text = field_str(object, name)?;
    if is_sha256_hex(text) {
        Ok(text.to_string())
    } else {
        Err(format!("{name} is not 64 lowercase hex characters"))
    }
}

fn field_bool(object: &Map<String, Value>, name: &str) -> Result<bool, String> {
    field(object, name)?
        .as_bool()
        .ok_or_else(|| format!("{name} is not a boolean"))
}

/// Parse one result entry for a check of the given spec. Validates exactly the fields the
/// statuses read.
fn observe(spec: &CheckSpec, value: &Value) -> Result<Observed, String> {
    let object = value.as_object().ok_or("the entry is not an object")?;
    if let Some(error) = object.get("error") {
        return error
            .as_str()
            .map(|text| Observed::Error(text.to_string()))
            .ok_or_else(|| "error is not a string".to_string());
    }
    match spec {
        CheckSpec::Sha256(_) => {
            let list = field(object, "entries")?
                .as_array()
                .ok_or("entries is not an array")?;
            let mut entries = Vec::with_capacity(list.len());
            for (k, item) in list.iter().enumerate() {
                let item = item
                    .as_object()
                    .ok_or_else(|| format!("entries[{k}] is not an object"))?;
                let path = field_str(item, "path")
                    .map_err(|e| format!("entries[{k}]: {e}"))?
                    .to_string();
                let kind =
                    match field_str(item, "type").map_err(|e| format!("entries[{k}]: {e}"))? {
                        "absent" => EntryType::Absent,
                        "file" => EntryType::File,
                        "symlink" => EntryType::Symlink,
                        "directory" => EntryType::Directory,
                        other => {
                            return Err(format!("entries[{k}].type {other:?} is not a known type"));
                        }
                    };
                let sha256 = match kind {
                    EntryType::File | EntryType::Symlink => Some(
                        field_str(item, "sha256")
                            .map_err(|e| format!("entries[{k}]: {e}"))?
                            .to_string(),
                    ),
                    _ => None,
                };
                entries.push(ShaEntry { path, kind, sha256 });
            }
            Ok(Observed::Sha {
                path_count: field_u64(object, "path_count")?,
                entries_sha256: field_digest(object, "entries_sha256")?,
                entries,
            })
        }
        CheckSpec::Json(_) => {
            let selected = field(object, "selected")?
                .as_object()
                .ok_or("selected is not an object")?
                .clone();
            Ok(Observed::Json {
                sha256: field_digest(object, "sha256")?,
                selected,
            })
        }
        CheckSpec::Grep(_) => {
            let matches = field(object, "matches")?
                .as_array()
                .ok_or("matches is not an array")?;
            let mut match_paths = Vec::with_capacity(matches.len());
            for (k, item) in matches.iter().enumerate() {
                let item = item
                    .as_object()
                    .ok_or_else(|| format!("matches[{k}] is not an object"))?;
                match_paths.push(
                    field_str(item, "path")
                        .map_err(|e| format!("matches[{k}]: {e}"))?
                        .to_string(),
                );
            }
            Ok(Observed::Grep {
                match_count: field_u64(object, "match_count")?,
                match_paths,
            })
        }
        CheckSpec::Git(_) => {
            let stderr = match object.get("stderr") {
                None => None,
                Some(Value::String(text)) => Some(text.clone()),
                Some(_) => return Err("stderr is not a string".to_string()),
            };
            Ok(Observed::Git {
                returncode: field(object, "returncode")?
                    .as_i64()
                    .ok_or("returncode is not an integer")?,
                output_sha256: field_digest(object, "output_sha256")?,
                byte_count: field_u64(object, "byte_count")?,
                text: field_str(object, "text")?.to_string(),
                truncated: field_bool(object, "truncated")?,
                stderr,
            })
        }
    }
}

/// Echo cross-checks (contract 5.2 item 4). Returns the binding level on success.
fn echo(spec: &CheckSpec, observed: &Observed) -> Result<EchoBinding, String> {
    match (spec, observed) {
        (_, Observed::Error(_)) => Ok(EchoBinding::None),
        (
            CheckSpec::Sha256(paths),
            Observed::Sha {
                path_count,
                entries,
                ..
            },
        ) => {
            if *path_count != paths.len() as u64 || entries.len() != paths.len() {
                return Err(format!(
                    "path_count {path_count} and {} entries, spec has {} paths",
                    entries.len(),
                    paths.len()
                ));
            }
            for (k, (entry, path)) in entries.iter().zip(paths).enumerate() {
                if &entry.path != path {
                    return Err(format!(
                        "entries[{k}].path is {:?}, spec has {path:?}",
                        entry.path
                    ));
                }
            }
            Ok(EchoBinding::EchoVerified)
        }
        (CheckSpec::Json(json), Observed::Json { selected, .. }) => {
            let want: BTreeSet<&str> = json.select.iter().map(String::as_str).collect();
            let have: BTreeSet<&str> = selected.keys().map(String::as_str).collect();
            if want == have {
                Ok(EchoBinding::EchoVerified)
            } else {
                Err(format!("selected keys {have:?}, spec selects {want:?}"))
            }
        }
        (
            CheckSpec::Grep(grep),
            Observed::Grep {
                match_count,
                match_paths,
            },
        ) => {
            for path in match_paths {
                if !grep.paths.contains(path) {
                    return Err(format!("match path {path:?} is not in the spec's paths"));
                }
            }
            if *match_count < match_paths.len() as u64 {
                return Err(format!(
                    "match_count {match_count} is below the {} matches listed",
                    match_paths.len()
                ));
            }
            Ok(EchoBinding::Partial)
        }
        (CheckSpec::Git(_), Observed::Git { .. }) => Ok(EchoBinding::None),
        _ => Err("the observed shape does not belong to the spec's kind".to_string()),
    }
}

// ---------------------------------------------------------------------------
// Run plan, observation and result views
// ---------------------------------------------------------------------------

struct PlanView {
    root: String,
    extra_roots: Vec<String>,
    checks: BTreeMap<String, Value>,
}

fn parse_run_plan(bytes: &[u8], errors: &mut Vec<ErrorItem>) -> Option<PlanView> {
    let malformed = |message: String| binding_error(bc::RUN_PLAN_MALFORMED, "run_plan", message);
    let value: Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(error) => {
            errors.push(malformed(format!("not JSON: {error}")));
            return None;
        }
    };
    let Some(object) = value.as_object() else {
        errors.push(malformed("not a JSON object".to_string()));
        return None;
    };
    let before = errors.len();
    let root = match object.get("root").and_then(Value::as_str) {
        Some(root) => root.to_string(),
        None => {
            errors.push(malformed("root is missing or not a string".to_string()));
            String::new()
        }
    };
    let mut extra_roots = Vec::new();
    match object.get("extra_roots") {
        None | Some(Value::Null) => {}
        Some(Value::Array(items)) => {
            for item in items {
                match item.as_str() {
                    Some(text) => extra_roots.push(text.to_string()),
                    None => errors.push(malformed("extra_roots holds a non-string".to_string())),
                }
            }
        }
        Some(_) => errors.push(malformed("extra_roots is not an array".to_string())),
    }
    let mut checks = BTreeMap::new();
    match object.get("checks").and_then(Value::as_array) {
        None => errors.push(malformed("checks is missing or not an array".to_string())),
        Some(items) => {
            for (n, item) in items.iter().enumerate() {
                let Some(id) = item.get("id").and_then(Value::as_str) else {
                    errors.push(malformed(format!(
                        "checks[{n}] is not an object with a string id"
                    )));
                    continue;
                };
                if checks.insert(id.to_string(), item.clone()).is_some() {
                    errors.push(binding_error(
                        bc::RUN_PLAN_DUPLICATE_ID,
                        format!("run_plan.checks[{n}].id"),
                        format!("check id {id:?} appears more than once"),
                    ));
                }
            }
        }
    }
    (errors.len() == before).then_some(PlanView {
        root,
        extra_roots,
        checks,
    })
}

struct ResultView {
    battery_sha256: String,
    failed: Vec<String>,
    checks: Map<String, Value>,
}

fn parse_result(bytes: &[u8], errors: &mut Vec<ErrorItem>) -> Option<ResultView> {
    let malformed = |message: String| binding_error(bc::RESULT_MALFORMED, "result", message);
    let value: Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(error) => {
            errors.push(malformed(format!("not JSON: {error}")));
            return None;
        }
    };
    let Some(object) = value.as_object() else {
        errors.push(malformed("not a JSON object".to_string()));
        return None;
    };
    let before = errors.len();
    let battery_sha256 = match object.get("battery_sha256").and_then(Value::as_str) {
        Some(text) => text.to_string(),
        None => {
            errors.push(malformed(
                "battery_sha256 is missing or not a string".to_string(),
            ));
            String::new()
        }
    };
    let mut failed = Vec::new();
    match object.get("failed").and_then(Value::as_array) {
        None => errors.push(malformed("failed is missing or not an array".to_string())),
        Some(items) => {
            for item in items {
                match item.as_str() {
                    Some(text) => failed.push(text.to_string()),
                    None => errors.push(malformed("failed holds a non-string".to_string())),
                }
            }
        }
    }
    let checks = match object.get("checks").and_then(Value::as_object) {
        Some(checks) => checks.clone(),
        None => {
            errors.push(malformed("checks is missing or not an object".to_string()));
            Map::new()
        }
    };
    (errors.len() == before).then_some(ResultView {
        battery_sha256,
        failed,
        checks,
    })
}

// ---------------------------------------------------------------------------
// evaluate
// ---------------------------------------------------------------------------

/// Evaluate one profile and subject against a battery result (contract 5.2, 5.3).
pub fn evaluate(inputs: &EvaluateInputs<'_>, read: ReadFile<'_>) -> Result<Evaluation, EvalError> {
    if !is_utc_timestamp(inputs.evaluated_at) {
        return Err(EvalError::Input(format!(
            "evaluated_at {:?} must be UTC YYYY-MM-DDTHH:MM:SSZ",
            inputs.evaluated_at
        )));
    }
    let profile_sha = sha256_hex(inputs.profile);
    let pairs = [PairBytes {
        profile: inputs.profile,
        subject: inputs.subject,
    }];
    let into_error = |error: PlanError| match error {
        PlanError::Input(message) => EvalError::Input(message),
        PlanError::Refused(errors) => EvalError::Invalid(errors),
    };
    let loaded = load_pairs(&pairs, read).map_err(into_error)?;
    let derivation = derive(&loaded).map_err(into_error)?;
    let pair = &loaded[0];

    let mut errors: Vec<ErrorItem> = Vec::new();
    let plan_sha = sha256_hex(inputs.run_plan.bytes);
    let battery_file_sha = sha256_hex(inputs.battery.bytes);
    let observation_sha = sha256_hex(inputs.observation.bytes);

    // Run plan: every derived check present and identical; roots.
    if let Some(view) = parse_run_plan(inputs.run_plan.bytes, &mut errors) {
        check_run_plan(&derivation, &view, &mut errors);
    }

    // Observation record.
    let observation = match parse_document::<Observation>(inputs.observation.bytes) {
        Ok(observation) => {
            for error in validate_observation(&observation) {
                errors.push(binding_error(
                    bc::OBSERVATION_INVALID,
                    format!("observation.{}", error.at),
                    format!("{}: {}", error.code, error.message),
                ));
            }
            if observation.plan.sha256 != plan_sha {
                errors.push(binding_error(
                    bc::OBSERVATION_PLAN_DIGEST_MISMATCH,
                    "observation.plan.sha256",
                    format!("the run plan file has sha256 {plan_sha}"),
                ));
            }
            if observation.result.sha256 != battery_file_sha {
                errors.push(binding_error(
                    bc::OBSERVATION_RESULT_DIGEST_MISMATCH,
                    "observation.result.sha256",
                    format!("the result file has sha256 {battery_file_sha}"),
                ));
            }
            Some(observation)
        }
        Err(DocError::Syntax(message)) | Err(DocError::Schema(message)) => {
            errors.push(binding_error(
                bc::OBSERVATION_INVALID,
                "observation",
                message,
            ));
            None
        }
    };

    // Result, and its agreement with the observation.
    let result = parse_result(inputs.battery.bytes, &mut errors);
    if let (Some(observation), Some(result)) = (&observation, &result) {
        if matches!(observation.battery_exit, 0 | 1)
            && (observation.battery_exit == 1) == result.failed.is_empty()
        {
            errors.push(binding_error(
                bc::BATTERY_EXIT_INCONSISTENT,
                "observation.battery_exit",
                format!(
                    "battery_exit is {} and the result's failed list holds {} id(s)",
                    observation.battery_exit,
                    result.failed.len()
                ),
            ));
        }
        if observation.stdout_battery_sha256 != result.battery_sha256 {
            errors.push(binding_error(
                bc::BATTERY_SHA256_MISMATCH,
                "observation.stdout_battery_sha256",
                "differs from the result's battery_sha256",
            ));
        }
    }

    // Result entries: presence, kind, shape and echo for every derived check.
    let mut observed: BTreeMap<String, Observed> = BTreeMap::new();
    let mut binding_checks: BTreeMap<String, EchoBinding> = BTreeMap::new();
    if let Some(result) = &result {
        for check in &derivation.plan.checks {
            let at = format!("result.checks.{}", check.id);
            let Some(entry) = result.checks.get(&check.id) else {
                errors.push(binding_error(
                    bc::RESULT_ENTRY_MISSING,
                    at,
                    "the result has no entry for this derived check",
                ));
                continue;
            };
            let kind = entry.get("kind").and_then(Value::as_str);
            if kind != Some(spec_kind(&check.spec)) {
                errors.push(binding_error(
                    bc::RESULT_KIND_MISMATCH,
                    at,
                    format!(
                        "entry kind is {kind:?}, the derived spec is {:?}",
                        spec_kind(&check.spec)
                    ),
                ));
                continue;
            }
            let parsed = match observe(&check.spec, entry) {
                Ok(parsed) => parsed,
                Err(message) => {
                    errors.push(binding_error(bc::RESULT_MALFORMED, at, message));
                    continue;
                }
            };
            match echo(&check.spec, &parsed) {
                Ok(level) => {
                    binding_checks.insert(check.id.clone(), level);
                    observed.insert(check.id.clone(), parsed);
                }
                Err(message) => errors.push(binding_error(bc::ECHO_MISMATCH, at, message)),
            }
        }
    }
    let (Some(observation), Some(result)) = (observation, result) else {
        return Err(EvalError::Binding(errors));
    };
    if !errors.is_empty() {
        return Err(EvalError::Binding(errors));
    }

    // From here every derived check has a validated, echo-checked observation.
    let specs: BTreeMap<&str, &PlanCheck> = derivation
        .plan
        .checks
        .iter()
        .map(|check| (check.id.as_str(), check))
        .collect();
    let context = Context {
        profile: &pair.profile,
        subject: &pair.subject,
        observed: &observed,
        specs: &specs,
    };
    let git_root = context.git_root_binding(&derivation);
    let mut results: Vec<EvaluatedRequirement> = Vec::new();
    let mut mechanical_evidence: BTreeMap<&str, Vec<EvidenceItem>> = BTreeMap::new();
    let predicate_checks: BTreeMap<(usize, usize), &PredicateChecks> = derivation
        .predicates
        .iter()
        .map(|record| ((record.requirement, record.predicate), record))
        .collect();
    for (r, requirement) in pair.profile.requirements.iter().enumerate() {
        match requirement.class {
            RequirementClass::Mechanical => {
                let mut predicates = Vec::new();
                let mut evidence = Vec::new();
                for (p, predicate) in requirement.predicates.iter().enumerate() {
                    let Some(record) = predicate_checks.get(&(r, p)) else {
                        // Every mechanical predicate derives checks; a miss is a defect.
                        return Err(EvalError::Input(format!(
                            "internal: no derived checks for requirement {} predicate {}",
                            requirement.id,
                            p + 1
                        )));
                    };
                    let outcome = context.predicate(predicate, record, &git_root);
                    predicates.push(PredicateResult {
                        index: p as u64 + 1,
                        kind: predicate.kind_name().to_string(),
                        status: outcome.status,
                        reason: outcome.reason,
                        detail: outcome.detail,
                    });
                    let ids = record
                        .main
                        .iter()
                        .chain(record.manifest.iter())
                        .chain(record.exists.iter());
                    for id in ids {
                        if let Some(item) = context.evidence(id) {
                            evidence.push(item);
                        }
                    }
                }
                let status = aggregate(predicates.iter().map(|p| p.status));
                mechanical_evidence.insert(requirement.id.as_str(), evidence.clone());
                results.push(EvaluatedRequirement {
                    id: requirement.id.clone(),
                    class: EvaluatedClass::Mechanical,
                    status,
                    reason: None,
                    predicates,
                    evidence,
                });
            }
            RequirementClass::Attributed => {
                let mut evidence: Vec<EvidenceItem> = Vec::new();
                for reference in &requirement.evidence_from {
                    for item in mechanical_evidence
                        .get(reference.as_str())
                        .into_iter()
                        .flatten()
                    {
                        if !evidence.iter().any(|seen| seen.check_id == item.check_id) {
                            evidence.push(item.clone());
                        }
                    }
                }
                results.push(EvaluatedRequirement {
                    id: requirement.id.clone(),
                    class: EvaluatedClass::Attributed,
                    status: Status::Indeterminate,
                    reason: Some(ATTRIBUTED_REASON.to_string()),
                    predicates: Vec::new(),
                    evidence,
                });
            }
        }
    }
    let mut counts = Counts {
        met: 0,
        unmet: 0,
        indeterminate: 0,
    };
    for requirement in &results {
        match requirement.status {
            Status::Met => counts.met += 1,
            Status::Unmet => counts.unmet += 1,
            Status::Indeterminate => counts.indeterminate += 1,
        }
    }
    Ok(Evaluation {
        schema: EVALUATION_SCHEMA.to_string(),
        profile: ProfileRef {
            id: pair.profile.profile_id.clone(),
            version: pair.profile.version,
            sha256: profile_sha,
        },
        subject: SubjectRef {
            subject_id: subject_id(inputs.subject),
            sha256: sha256_hex(inputs.subject),
        },
        run_plan: FileRef {
            path: inputs.run_plan.path.to_string(),
            sha256: plan_sha,
        },
        battery: BatteryRecord {
            path: inputs.battery.path.to_string(),
            file_sha256: battery_file_sha,
            battery_sha256: result.battery_sha256,
            failed: result.failed,
        },
        observation: ObservationRecord {
            path: inputs.observation.path.to_string(),
            sha256: observation_sha,
            started_at: observation.started_at,
            finished_at: observation.finished_at,
            battery_exit: observation.battery_exit,
            battery_script_sha256: observation.battery_script.sha256,
        },
        tool: ToolRecord {
            name: TOOL_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            binary_sha256: inputs.binary_sha256.to_string(),
        },
        evaluated_at: inputs.evaluated_at.to_string(),
        derived_check_ids: derivation
            .plan
            .checks
            .iter()
            .map(|check| check.id.clone())
            .collect(),
        binding: Binding {
            git_root: git_root.binding(),
            checks: binding_checks,
        },
        requirements: results,
        counts,
    })
}

/// The subject's id, read from the already-validated subject bytes.
fn subject_id(bytes: &[u8]) -> String {
    serde_json::from_slice::<Value>(bytes)
        .ok()
        .and_then(|value| value.get("subject_id")?.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn check_run_plan(derivation: &Derivation, view: &PlanView, errors: &mut Vec<ErrorItem>) {
    for check in &derivation.plan.checks {
        let derived = serde_json::to_value(check).unwrap_or(Value::Null);
        match view.checks.get(&check.id) {
            None => errors.push(binding_error(
                bc::DERIVED_CHECK_MISSING,
                format!("run_plan.checks.{}", check.id),
                "the run plan omits this derived check",
            )),
            Some(found) if found != &derived => errors.push(binding_error(
                bc::DERIVED_CHECK_ALTERED,
                format!("run_plan.checks.{}", check.id),
                "the run plan's check differs from the derived check",
            )),
            Some(_) => {}
        }
    }
    let has_git_root = derivation.git_root_checks.iter().any(Option::is_some);
    if has_git_root && view.root != derivation.plan.root {
        errors.push(binding_error(
            bc::RUN_PLAN_ROOT_MISMATCH,
            "run_plan.root",
            format!(
                "root is {:?}; the profile declares a git root, so it must be {:?}",
                view.root, derivation.plan.root
            ),
        ));
    }
    let planned: BTreeSet<&str> = std::iter::once(view.root.as_str())
        .chain(view.extra_roots.iter().map(String::as_str))
        .collect();
    for path in std::iter::once(&derivation.plan.root).chain(&derivation.plan.extra_roots) {
        if !planned.contains(path.as_str()) {
            errors.push(binding_error(
                bc::RUN_PLAN_ROOT_MISSING,
                "run_plan.extra_roots",
                format!("derived root {path:?} is in neither root nor extra_roots"),
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// Statuses (contract 3.6)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GitRoot {
    Verified,
    Mismatch,
    NotApplicable,
}

impl GitRoot {
    fn binding(self) -> GitRootBinding {
        match self {
            GitRoot::Verified => GitRootBinding::Verified,
            GitRoot::Mismatch => GitRootBinding::Mismatch,
            GitRoot::NotApplicable => GitRootBinding::NotApplicable,
        }
    }
}

struct Outcome {
    status: Status,
    reason: Option<String>,
    detail: Option<String>,
}

impl Outcome {
    fn met() -> Self {
        Outcome {
            status: Status::Met,
            reason: None,
            detail: None,
        }
    }
    fn with(status: Status, reason: &str, detail: Option<String>) -> Self {
        debug_assert!(PREDICATE_REASONS.contains(&reason));
        Outcome {
            status,
            reason: Some(reason.to_string()),
            detail: detail.map(|text| cut(&text, 400)),
        }
    }
    fn unmet(reason: &str, detail: impl Into<Option<String>>) -> Self {
        Self::with(Status::Unmet, reason, detail.into())
    }
    fn indeterminate(reason: &str, detail: impl Into<Option<String>>) -> Self {
        Self::with(Status::Indeterminate, reason, detail.into())
    }
}

/// `text` cut to at most `limit` characters.
fn cut(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}

fn aggregate(statuses: impl Iterator<Item = Status> + Clone) -> Status {
    if statuses.clone().any(|s| s == Status::Unmet) {
        Status::Unmet
    } else if statuses.clone().any(|s| s == Status::Indeterminate) {
        Status::Indeterminate
    } else {
        Status::Met
    }
}

struct Context<'a> {
    profile: &'a Profile,
    subject: &'a crate::validate::TypedSubject,
    observed: &'a BTreeMap<String, Observed>,
    specs: &'a BTreeMap<&'a str, &'a PlanCheck>,
}

fn trim_line_end(text: &str) -> &str {
    text.trim_end_matches(['\n', '\r'])
}

impl Context<'_> {
    fn get(&self, id: &str) -> Option<&Observed> {
        self.observed.get(id)
    }

    fn subject_git_root(&self) -> Option<&str> {
        self.profile
            .roots
            .iter()
            .find(|root| root.git)
            .and_then(|root| self.subject.roots.get(&root.name))
            .map(String::as_str)
    }

    /// Rule 0 (contract 3.4 rule 4): is the plan's git root the subject's git root.
    fn git_root_binding(&self, derivation: &Derivation) -> GitRootState {
        let Some(id) = derivation.git_root_checks.first().and_then(Option::as_ref) else {
            return GitRootState::new(GitRoot::NotApplicable, String::new());
        };
        let want = self.subject_git_root().unwrap_or_default();
        match self.get(id) {
            Some(Observed::Git {
                returncode: 0,
                text,
                ..
            }) if trim_line_end(text) == want => {
                GitRootState::new(GitRoot::Verified, String::new())
            }
            Some(Observed::Git {
                returncode: 0,
                text,
                ..
            }) => GitRootState::new(
                GitRoot::Mismatch,
                format!(
                    "git toplevel is {:?}; the subject's git root is {want:?}",
                    trim_line_end(text)
                ),
            ),
            Some(Observed::Git { returncode, .. }) => GitRootState::new(
                GitRoot::Mismatch,
                format!(
                    "git rev-parse --show-toplevel exited {returncode}; the subject's git root is {want:?}"
                ),
            ),
            Some(Observed::Error(error)) => GitRootState::new(
                GitRoot::Mismatch,
                format!(
                    "the git root check errored ({}); the subject's git root is {want:?}",
                    cut(error, 200)
                ),
            ),
            _ => GitRootState::new(
                GitRoot::Mismatch,
                "the git root check is missing".to_string(),
            ),
        }
    }

    fn evidence(&self, id: &str) -> Option<EvidenceItem> {
        let (kind, digest_field, digest) = match self.get(id)? {
            Observed::Sha { entries_sha256, .. } => ("sha256", "entries_sha256", entries_sha256),
            Observed::Json { sha256, .. } => ("json", "sha256", sha256),
            Observed::Git { output_sha256, .. } => ("git", "output_sha256", output_sha256),
            Observed::Grep { .. } | Observed::Error(_) => return None,
        };
        Some(EvidenceItem {
            check_id: id.to_string(),
            kind: kind.to_string(),
            digest_field: digest_field.to_string(),
            digest: digest.clone(),
        })
    }

    /// `path` relative to the longest subject root that contains it.
    fn relative<'p>(&self, path: &'p str) -> &'p str {
        self.subject
            .roots
            .values()
            .filter_map(|root| {
                path.strip_prefix(root.as_str())
                    .and_then(|rest| rest.strip_prefix('/'))
                    .map(|rest| (root.len(), rest))
            })
            .max_by_key(|(len, _)| *len)
            .map_or(path, |(_, rest)| rest)
    }

    fn str_value(&self, value: &StrVal) -> Option<String> {
        match value {
            StrVal::Literal(text) => Some(text.clone()),
            StrVal::Param(param) => match self.subject.values.get(&param.param) {
                Some(TypedValue::Str(text)) => Some(text.clone()),
                _ => None,
            },
        }
    }

    fn predicate(
        &self,
        predicate: &Predicate,
        record: &PredicateChecks,
        git_root: &GitRootState,
    ) -> Outcome {
        // Rule 0: a git predicate cannot be read against a toplevel that is not the subject's.
        if predicate.is_git() && git_root.state == GitRoot::Mismatch {
            return Outcome::indeterminate("git_root_mismatch", git_root.detail.clone());
        }
        // Existence set per predicate.
        let existence: Vec<&String> = match predicate {
            Predicate::FileSha256(_) => record.main.iter().collect(),
            Predicate::ManifestMatch(_) => record.main.iter().chain(&record.manifest).collect(),
            Predicate::GitOutputEquals(_) | Predicate::GitOutputEmpty(_) => Vec::new(),
            _ => record.exists.iter().collect(),
        };
        // Rule 1.
        for id in &existence {
            if let Some(Observed::Error(error)) = self.get(id) {
                return Outcome::indeterminate(
                    "existence_check_error",
                    format!("{id}: {}", cut(error, 200)),
                );
            }
        }
        // Rule 2.
        let mut offending: Vec<(&str, EntryType)> = Vec::new();
        for id in &existence {
            if let Some(Observed::Sha { entries, .. }) = self.get(id) {
                offending.extend(
                    entries
                        .iter()
                        .filter(|entry| entry.kind != EntryType::File)
                        .map(|entry| (entry.path.as_str(), entry.kind)),
                );
            }
        }
        if let Some(&(_, first)) = offending.first() {
            let reason = match first {
                EntryType::Absent => "absent",
                EntryType::Directory => "directory",
                _ => "symlink",
            };
            let shown: Vec<String> = offending
                .iter()
                .take(20)
                .map(|(path, kind)| {
                    format!(
                        "{} ({})",
                        self.relative(path),
                        match kind {
                            EntryType::Absent => "absent",
                            EntryType::Directory => "directory",
                            EntryType::Symlink => "symlink",
                            EntryType::File => "file",
                        }
                    )
                })
                .collect();
            let more = if offending.len() > 20 { ", ..." } else { "" };
            return Outcome::unmet(
                reason,
                format!(
                    "{} not a regular file: {}{more}",
                    offending.len(),
                    shown.join(", ")
                ),
            );
        }
        // Rule 3 (json, grep, git).
        if !matches!(
            predicate,
            Predicate::FileSha256(_) | Predicate::ManifestMatch(_)
        ) {
            for id in &record.main {
                if let Some(Observed::Error(error)) = self.get(id) {
                    return Outcome::indeterminate(
                        "check_error",
                        format!("{id}: {}", cut(error, 200)),
                    );
                }
            }
        }
        // Rule 4 (json).
        if matches!(
            predicate,
            Predicate::JsonEquals(_) | Predicate::JsonCompare(_) | Predicate::JsonPresent(_)
        ) {
            let main = record.main.first().and_then(|id| self.get(id));
            let exists = record.exists.as_ref().and_then(|id| self.get(id));
            if let (Some(Observed::Json { sha256, .. }), Some(Observed::Sha { entries, .. })) =
                (main, exists)
                && let Some(entry_sha) = entries.first().and_then(|e| e.sha256.as_deref())
                && entry_sha != sha256
            {
                return Outcome::indeterminate(
                    "observation_inconsistent",
                    format!("the json check read {sha256}; the existence check read {entry_sha}"),
                );
            }
        }
        // Rule 5.
        self.rule5(predicate, record)
    }

    fn rule5(&self, predicate: &Predicate, record: &PredicateChecks) -> Outcome {
        match predicate {
            Predicate::FileSha256(p) => {
                let (Some(expected), Some(Observed::Sha { entries, .. })) = (
                    self.str_value(&p.expected),
                    record.main.first().and_then(|id| self.get(id)),
                ) else {
                    return Outcome::indeterminate(
                        "check_error",
                        "the expected digest or the observation is not available".to_string(),
                    );
                };
                let actual = entries.first().and_then(|e| e.sha256.as_deref());
                if actual == Some(expected.as_str()) {
                    Outcome::met()
                } else {
                    Outcome::unmet(
                        "digest_differs",
                        format!("expected {expected}, observed {}", actual.unwrap_or("none")),
                    )
                }
            }
            Predicate::ManifestMatch(p) => self.manifest(&p.manifest.param, record),
            Predicate::JsonEquals(_) | Predicate::JsonCompare(_) | Predicate::JsonPresent(_) => {
                self.json(predicate, record)
            }
            Predicate::GrepCount(p) => {
                let mut failing: Vec<String> = Vec::new();
                for id in &record.main {
                    let Some(Observed::Grep { match_count, .. }) = self.get(id) else {
                        return Outcome::indeterminate(
                            "check_error",
                            format!("{id}: no grep observation"),
                        );
                    };
                    let below = p.min.is_some_and(|min| *match_count < min);
                    let above = p.max.is_some_and(|max| *match_count > max);
                    if below || above {
                        let path = match self.specs.get(id.as_str()).map(|c| &c.spec) {
                            Some(CheckSpec::Grep(grep)) if p.per_path => grep
                                .paths
                                .first()
                                .map(|path| format!("{} ", self.relative(path)))
                                .unwrap_or_default(),
                            _ => String::new(),
                        };
                        failing.push(format!("{path}count {match_count}"));
                    }
                }
                if failing.is_empty() {
                    Outcome::met()
                } else {
                    Outcome::unmet(
                        "count_out_of_bound",
                        format!(
                            "{} outside [{}, {}]",
                            failing.join(", "),
                            p.min.map_or("-".to_string(), |v| v.to_string()),
                            p.max.map_or("-".to_string(), |v| v.to_string())
                        ),
                    )
                }
            }
            Predicate::GitOutputEquals(p) => self.git(record, Some(&p.expected)),
            Predicate::GitOutputEmpty(_) => self.git(record, None),
        }
    }

    /// `git_output_equals` when `expected` is given, `git_output_empty` otherwise.
    fn git(&self, record: &PredicateChecks, expected: Option<&StrVal>) -> Outcome {
        let Some(Observed::Git {
            returncode,
            byte_count,
            text,
            truncated,
            stderr,
            ..
        }) = record.main.first().and_then(|id| self.get(id))
        else {
            return Outcome::indeterminate("check_error", "no git observation".to_string());
        };
        if *returncode != 0 {
            let detail = match stderr {
                Some(stderr) => format!("exit {returncode}: {}", cut(stderr.trim_end(), 200)),
                None => format!("exit {returncode}"),
            };
            return Outcome::indeterminate("git_nonzero", detail);
        }
        match expected {
            Some(expected) => {
                if *truncated {
                    return Outcome::indeterminate(
                        "output_truncated",
                        format!("{byte_count} byte(s) of output exceed the text limit"),
                    );
                }
                let Some(expected) = self.str_value(expected) else {
                    return Outcome::indeterminate(
                        "check_error",
                        "the expected value is not available".to_string(),
                    );
                };
                if trim_line_end(text) == expected {
                    Outcome::met()
                } else {
                    Outcome::unmet(
                        "differs",
                        "the git output differs from expected".to_string(),
                    )
                }
            }
            None => {
                if *byte_count == 0 {
                    Outcome::met()
                } else {
                    Outcome::unmet("not_empty", format!("{byte_count} byte(s) of output"))
                }
            }
        }
    }

    fn manifest(&self, param: &str, record: &PredicateChecks) -> Outcome {
        let (Some(TypedValue::Manifest(value)), Some(entries)) = (
            self.subject.values.get(param),
            self.subject.manifests.get(param),
        ) else {
            return Outcome::indeterminate(
                "check_error",
                format!("manifest parameter {param} is not available"),
            );
        };
        let main = record.main.first().and_then(|id| self.get(id));
        let manifest = record.manifest.as_ref().and_then(|id| self.get(id));
        let (Some(Observed::Sha { entries: seen, .. }), Some(Observed::Sha { entries: file, .. })) =
            (main, manifest)
        else {
            return Outcome::indeterminate("check_error", "no manifest observation".to_string());
        };
        // E5: the manifest file the battery read must be the one the subject names.
        let manifest_sha = file.first().and_then(|e| e.sha256.as_deref());
        if manifest_sha != Some(value.sha256.as_str()) {
            return Outcome::indeterminate(
                "manifest_changed",
                format!(
                    "the battery read manifest {}; the subject names {}",
                    manifest_sha.unwrap_or("none"),
                    value.sha256
                ),
            );
        }
        let differing: Vec<&str> = entries
            .iter()
            .zip(seen)
            .filter(|(entry, observed)| observed.sha256.as_deref() != Some(entry.sha256.as_str()))
            .map(|(entry, _)| entry.path.as_str())
            .collect();
        if differing.is_empty() {
            Outcome::met()
        } else {
            let more = if differing.len() > 20 { ", ..." } else { "" };
            Outcome::unmet(
                "digest_differs",
                format!(
                    "{} of {} entries differ: {}{more}",
                    differing.len(),
                    entries.len(),
                    differing
                        .iter()
                        .take(20)
                        .copied()
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
        }
    }

    fn json(&self, predicate: &Predicate, record: &PredicateChecks) -> Outcome {
        let Some(id) = record.main.first() else {
            return Outcome::indeterminate("check_error", "no json check".to_string());
        };
        let (Some(Observed::Json { selected, .. }), Some(check)) =
            (self.get(id), self.specs.get(id.as_str()))
        else {
            return Outcome::indeterminate("check_error", format!("{id}: no json observation"));
        };
        let CheckSpec::Json(spec) = &check.spec else {
            return Outcome::indeterminate("check_error", format!("{id}: not a json check"));
        };
        let expected: Option<Value> = match predicate {
            Predicate::JsonEquals(p) => match &p.expected {
                Expected::Literal(value) => Some(value.clone()),
                Expected::Param(param) => match self.subject.values.get(&param.param) {
                    Some(TypedValue::Str(text)) => Some(Value::String(text.clone())),
                    _ => {
                        return Outcome::indeterminate(
                            "check_error",
                            format!("parameter {} has no string value", param.param),
                        );
                    }
                },
            },
            _ => None,
        };
        for key in &spec.select {
            let value = selected.get(key).unwrap_or(&Value::Null);
            if value.is_null() {
                return Outcome::unmet("null_or_missing", format!("key {key}"));
            }
            match predicate {
                Predicate::JsonPresent(_) => {}
                Predicate::JsonEquals(_) => {
                    let Some(expected) = &expected else { continue };
                    if json_type(value) != json_type(expected) {
                        return Outcome::unmet(
                            "type_mismatch",
                            format!(
                                "key {key}: selected a {}, expected a {}",
                                json_type(value),
                                json_type(expected)
                            ),
                        );
                    }
                    if value != expected {
                        return Outcome::unmet("differs", format!("key {key}"));
                    }
                }
                Predicate::JsonCompare(p) => {
                    let Value::Number(number) = value else {
                        return Outcome::unmet(
                            "type_mismatch",
                            format!(
                                "key {key}: selected a {}, expected a number",
                                json_type(value)
                            ),
                        );
                    };
                    let ordering = compare_numbers(number, &p.value);
                    let holds = match (p.op, ordering) {
                        (CompareOp::Eq, Some(o)) => o == Ordering::Equal,
                        (CompareOp::Le, Some(o)) => o != Ordering::Greater,
                        (CompareOp::Ge, Some(o)) => o != Ordering::Less,
                        (_, None) => false,
                    };
                    if !holds {
                        let reason = if p.op == CompareOp::Eq {
                            "differs"
                        } else {
                            "out_of_bound"
                        };
                        return Outcome::unmet(reason, format!("key {key}"));
                    }
                }
                _ => {}
            }
        }
        Outcome::met()
    }
}

struct GitRootState {
    state: GitRoot,
    detail: Option<String>,
}

impl GitRootState {
    fn new(state: GitRoot, detail: String) -> Self {
        GitRootState {
            state,
            detail: (!detail.is_empty()).then_some(detail),
        }
    }
    fn binding(&self) -> GitRootBinding {
        self.state.binding()
    }
}

fn json_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn as_i128(number: &Number) -> Option<i128> {
    number
        .as_i64()
        .map(i128::from)
        .or_else(|| number.as_u64().map(i128::from))
}

/// Integers compare exactly; if either side is not an integer both compare as `f64`
/// (the representation in the document decides, not the value).
fn compare_numbers(left: &Number, right: &Number) -> Option<Ordering> {
    match (as_i128(left), as_i128(right)) {
        (Some(a), Some(b)) => Some(a.cmp(&b)),
        _ => left.as_f64()?.partial_cmp(&right.as_f64()?),
    }
}
