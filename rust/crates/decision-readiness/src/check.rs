//! `check --kind evaluation` and `check --kind assessment` (contract 3 and 6), `--rederive`
//! comparison, and `recheck`. Pure functions over parsed documents and file bytes.
//!
//! The tool validates assessments and never writes one: `Assessment` is `Deserialize` only
//! and nothing here constructs it.

use crate::diag::{ErrorItem, codes};
use crate::digest::{is_sha256_hex, sha256_hex};
use crate::evaluate::{ATTRIBUTED_REASON, PREDICATE_REASONS};
use crate::model::*;
use crate::validate::{is_normalised_absolute, is_utc_timestamp};
use serde_json::Value;
use std::collections::BTreeMap;

/// Codes added for these checks (stable; a code never changes meaning).
pub mod check_codes {
    pub const COUNTS_INCONSISTENT: &str = "counts_inconsistent";
    pub const STATUS_INCONSISTENT: &str = "status_inconsistent";
    pub const ATTRIBUTED_SHAPE: &str = "attributed_shape";
    pub const REASON_INVALID: &str = "reason_invalid";
    pub const DETAIL_TOO_LONG: &str = "detail_too_long";
    pub const EVIDENCE_INVALID: &str = "evidence_invalid";
    pub const REDERIVE_MISMATCH: &str = "rederive_mismatch";
    pub const TOOL_DIFFERS: &str = "tool_differs";
    pub const PROFILE_FILE_DIGEST_MISMATCH: &str = "profile_file_digest_mismatch";
    pub const SUBJECT_FILE_DIGEST_MISMATCH: &str = "subject_file_digest_mismatch";
    pub const RUN_PLAN_FILE_DIGEST_MISMATCH: &str = "run_plan_file_digest_mismatch";
    pub const BATTERY_FILE_DIGEST_MISMATCH: &str = "battery_file_digest_mismatch";
    pub const OBSERVATION_FILE_DIGEST_MISMATCH: &str = "observation_file_digest_mismatch";
    pub const EVALUATION_DIGEST_MISMATCH: &str = "evaluation_digest_mismatch";
    pub const EVALUATION_PROFILE_MISMATCH: &str = "evaluation_profile_mismatch";
    pub const DECISION_MISMATCH: &str = "decision_mismatch";
    pub const REQUIREMENT_MISSING: &str = "requirement_missing";
    pub const REQUIREMENT_UNKNOWN: &str = "requirement_unknown";
    pub const REQUIREMENT_DUPLICATE: &str = "requirement_duplicate";
    pub const EVALUATION_STATUS_MISMATCH: &str = "evaluation_status_mismatch";
    pub const MECHANICAL_UPGRADE: &str = "mechanical_upgrade";
    pub const DOWNGRADE_BASIS_NOT_ASSESSOR: &str = "downgrade_basis_not_assessor";
    pub const DOWNGRADE_REASON_MISSING: &str = "downgrade_reason_missing";
    pub const DOWNGRADE_EVIDENCE_MISSING: &str = "downgrade_evidence_missing";
    pub const ATTRIBUTED_REASON_MISSING: &str = "attributed_reason_missing";
    pub const ATTRIBUTED_EVIDENCE_MISSING: &str = "attributed_evidence_missing";
    pub const VERDICT_INCONSISTENT: &str = "verdict_inconsistent";
    pub const MISSING_SET_MISMATCH: &str = "missing_set_mismatch";
    pub const INDETERMINATE_SET_MISMATCH: &str = "indeterminate_set_mismatch";
    pub const ASSESSOR_ROLE_NOT_ALLOWED: &str = "assessor_role_not_allowed";
    pub const AUTHORITY_REF_EMPTY: &str = "authority_ref_empty";
    pub const FRESHNESS_BASIS_MISMATCH: &str = "freshness_basis_mismatch";
    pub const CLAIM_RELIANCE_INVALID: &str = "claim_reliance_invalid";
    pub const EVALUATION_INVALID: &str = "evaluation_invalid";
}
use check_codes as cc;

const KINDS: [&str; 8] = [
    "file_sha256",
    "manifest_match",
    "json_equals",
    "json_compare",
    "json_present",
    "grep_count",
    "git_output_equals",
    "git_output_empty",
];

const EVIDENCE_PAIRS: [(&str, &str); 3] = [
    ("sha256", "entries_sha256"),
    ("json", "sha256"),
    ("git", "output_sha256"),
];

fn err(code: &str, at: impl Into<String>, message: impl Into<String>) -> ErrorItem {
    ErrorItem::new(code, at, message)
}

/// Requirement status from its predicate statuses: any unmet, else any indeterminate, else met.
fn aggregate(statuses: impl Iterator<Item = Status> + Clone) -> Status {
    if statuses.clone().any(|s| s == Status::Unmet) {
        Status::Unmet
    } else if statuses.clone().any(|s| s == Status::Indeterminate) {
        Status::Indeterminate
    } else {
        Status::Met
    }
}

/// `YYYY-MM-DDTHH:MM:SS[.fraction]Z`.
pub fn is_rfc3339_utc(text: &str) -> bool {
    let Some(body) = text.strip_suffix('Z') else {
        return false;
    };
    match body.split_once('.') {
        None => is_utc_timestamp(text),
        Some((seconds, fraction)) => {
            !fraction.is_empty()
                && fraction.bytes().all(|b| b.is_ascii_digit())
                && is_utc_timestamp(&format!("{seconds}Z"))
        }
    }
}

// ---------------------------------------------------------------------------
// check --kind evaluation: structural rules
// ---------------------------------------------------------------------------

pub fn check_evaluation(e: &Evaluation) -> Vec<ErrorItem> {
    let mut errors = Vec::new();
    if e.schema != EVALUATION_SCHEMA {
        errors.push(err(
            codes::SCHEMA_MISMATCH,
            "schema",
            format!("expected {EVALUATION_SCHEMA}"),
        ));
    }
    for (at, sha) in [
        ("profile.sha256", &e.profile.sha256),
        ("subject.sha256", &e.subject.sha256),
        ("run_plan.sha256", &e.run_plan.sha256),
        ("battery.file_sha256", &e.battery.file_sha256),
        ("battery.battery_sha256", &e.battery.battery_sha256),
        ("observation.sha256", &e.observation.sha256),
        (
            "observation.battery_script_sha256",
            &e.observation.battery_script_sha256,
        ),
        ("tool.binary_sha256", &e.tool.binary_sha256),
    ] {
        if !is_sha256_hex(sha) {
            errors.push(err(
                codes::INVALID_SHA256,
                at,
                "not 64 lowercase hex digits",
            ));
        }
    }
    for (at, path) in [
        ("run_plan.path", &e.run_plan.path),
        ("battery.path", &e.battery.path),
        ("observation.path", &e.observation.path),
    ] {
        if !is_normalised_absolute(path) {
            errors.push(err(
                codes::INVALID_ABSOLUTE_PATH,
                at,
                "not an absolute normalised path",
            ));
        }
    }
    for (at, time) in [
        ("evaluated_at", &e.evaluated_at),
        ("observation.started_at", &e.observation.started_at),
        ("observation.finished_at", &e.observation.finished_at),
    ] {
        if !is_utc_timestamp(time) {
            errors.push(err(
                codes::INVALID_TIMESTAMP,
                at,
                "not YYYY-MM-DDTHH:MM:SSZ",
            ));
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut counts = Counts {
        met: 0,
        unmet: 0,
        indeterminate: 0,
    };
    for (i, r) in e.requirements.iter().enumerate() {
        let at = format!("requirements[{i}]");
        if !seen.insert(r.id.as_str()) {
            errors.push(err(
                codes::DUPLICATE_REQUIREMENT_ID,
                format!("{at}.id"),
                &r.id,
            ));
        }
        match r.status {
            Status::Met => counts.met += 1,
            Status::Unmet => counts.unmet += 1,
            Status::Indeterminate => counts.indeterminate += 1,
        }
        match r.class {
            EvaluatedClass::Attributed => {
                let ok = r.status == Status::Indeterminate
                    && r.reason.as_deref() == Some(ATTRIBUTED_REASON)
                    && r.predicates.is_empty();
                if !ok {
                    errors.push(err(
                        cc::ATTRIBUTED_SHAPE,
                        &at,
                        format!("an attributed requirement is indeterminate, reason {ATTRIBUTED_REASON}, with no predicates"),
                    ));
                }
            }
            EvaluatedClass::Mechanical => {
                if r.reason.is_some() || r.predicates.is_empty() {
                    errors.push(err(
                        cc::STATUS_INCONSISTENT,
                        &at,
                        "a mechanical requirement has predicates and no requirement-level reason",
                    ));
                } else if aggregate(r.predicates.iter().map(|p| p.status)) != r.status {
                    errors.push(err(
                        cc::STATUS_INCONSISTENT,
                        format!("{at}.status"),
                        "differs from the aggregate of its predicate statuses",
                    ));
                }
            }
        }
        for (j, p) in r.predicates.iter().enumerate() {
            let pat = format!("{at}.predicates[{j}]");
            if !KINDS.contains(&p.kind.as_str()) {
                errors.push(err(
                    cc::REASON_INVALID,
                    format!("{pat}.kind"),
                    "unknown predicate kind",
                ));
            }
            let reason_ok = match (&p.reason, p.status) {
                (None, Status::Met) => true,
                (Some(reason), Status::Unmet | Status::Indeterminate) => {
                    PREDICATE_REASONS.contains(&reason.as_str())
                }
                _ => false,
            };
            if !reason_ok {
                errors.push(err(
                    cc::REASON_INVALID,
                    format!("{pat}.reason"),
                    "present exactly when the status is not met, and from the closed reason set",
                ));
            }
            if p.detail.as_deref().is_some_and(|d| d.chars().count() > 400) {
                errors.push(err(
                    cc::DETAIL_TOO_LONG,
                    format!("{pat}.detail"),
                    "over 400 characters",
                ));
            }
        }
        for (k, item) in r.evidence.iter().enumerate() {
            let eat = format!("{at}.evidence[{k}]");
            if !EVIDENCE_PAIRS.contains(&(item.kind.as_str(), item.digest_field.as_str())) {
                errors.push(err(
                    cc::EVIDENCE_INVALID,
                    &eat,
                    "kind and digest_field are not a valid pairing",
                ));
            }
            if !is_sha256_hex(&item.digest) {
                errors.push(err(
                    codes::INVALID_SHA256,
                    format!("{eat}.digest"),
                    "not 64 lowercase hex digits",
                ));
            }
        }
    }
    if counts != e.counts {
        errors.push(err(
            cc::COUNTS_INCONSISTENT,
            "counts",
            "differ from the recount over requirements",
        ));
    }
    errors
}

/// Compare a recorded evaluation with a fresh one. Returns `(errors, notes)`: a mismatch in
/// `derived_check_ids`, `binding`, `requirements` or `counts` is an error; a differing `tool`
/// block is only a note.
pub fn compare_evaluations(
    recorded: &Evaluation,
    fresh: &Evaluation,
) -> (Vec<ErrorItem>, Vec<ErrorItem>) {
    let (mut errors, mut notes) = (Vec::new(), Vec::new());
    let value = |e: &Evaluation| serde_json::to_value(e).unwrap_or(Value::Null);
    let (a, b) = (value(recorded), value(fresh));
    for field in ["derived_check_ids", "binding", "requirements", "counts"] {
        if a[field] != b[field] {
            errors.push(err(
                cc::REDERIVE_MISMATCH,
                field,
                "the re-derived value differs from the recorded one",
            ));
        }
    }
    if a["tool"] != b["tool"] {
        notes.push(err(
            cc::TOOL_DIFFERS,
            "tool",
            "the recorded tool block differs from this binary",
        ));
    }
    (errors, notes)
}

// ---------------------------------------------------------------------------
// check --kind assessment
// ---------------------------------------------------------------------------

fn severity(status: Status) -> u8 {
    match status {
        Status::Met => 0,
        Status::Indeterminate => 1,
        Status::Unmet => 2,
    }
}

fn sorted(list: &[String]) -> Vec<&str> {
    let mut ids: Vec<&str> = list.iter().map(String::as_str).collect();
    ids.sort_unstable();
    ids
}

fn verdict_rank(verdict: VerdictValue) -> u8 {
    match verdict {
        VerdictValue::Ready => 0,
        VerdictValue::Uncertain => 1,
        VerdictValue::Blocked => 2,
    }
}

/// Everything an assessment is checked against.
pub struct AssessmentContext<'a> {
    pub profile: &'a Profile,
    pub profile_bytes: &'a [u8],
    pub evaluation: &'a Evaluation,
    pub evaluation_bytes: &'a [u8],
}

pub fn check_assessment(a: &Assessment, ctx: &AssessmentContext<'_>) -> Vec<ErrorItem> {
    // Errata E9: the evaluation the assessment cites is structurally checked first. Every rule
    // below compares against its statuses, so none is meaningful over an inconsistent one.
    let structural = check_evaluation(ctx.evaluation);
    if !structural.is_empty() {
        let mut errors = vec![err(
            cc::EVALUATION_INVALID,
            "--evaluation",
            "the evaluation fails check --kind evaluation; the underlying codes follow",
        )];
        errors.extend(structural);
        return errors;
    }
    let mut errors = Vec::new();
    let profile = ctx.profile;
    let evaluation = ctx.evaluation;
    if a.schema != ASSESSMENT_SCHEMA {
        errors.push(err(
            codes::SCHEMA_MISMATCH,
            "schema",
            format!("expected {ASSESSMENT_SCHEMA}"),
        ));
    }
    // Added shape rules (errata E7).
    if !is_rfc3339_utc(&a.issued_at) {
        errors.push(err(
            codes::INVALID_TIMESTAMP,
            "issued_at",
            "not RFC3339 UTC",
        ));
    }
    if a.assessment_id.trim().is_empty() {
        errors.push(err(codes::EMPTY_NAME, "assessment_id", "empty"));
    }
    if a.assessor.name.trim().is_empty() {
        errors.push(err(codes::EMPTY_NAME, "assessor.name", "empty"));
    }
    for (at, file) in std::iter::once(("evaluation", &a.evaluation))
        .chain(a.evidence.iter().map(|f| ("evidence", f)))
    {
        if !is_normalised_absolute(&file.path) {
            errors.push(err(
                codes::INVALID_ABSOLUTE_PATH,
                format!("{at}.path"),
                &file.path,
            ));
        }
        if !is_sha256_hex(&file.sha256) {
            errors.push(err(
                codes::INVALID_SHA256,
                format!("{at}.sha256"),
                "not 64 lowercase hex digits",
            ));
        }
    }
    for (i, c) in a.freshness_basis.claim_reliance.iter().enumerate() {
        if c.claim_id.trim().is_empty() || c.revision.trim().is_empty() {
            errors.push(err(
                cc::CLAIM_RELIANCE_INVALID,
                format!("freshness_basis.claim_reliance[{i}]"),
                "claim_id and revision are non-empty",
            ));
        }
    }

    // Rule 1: digests and decision.
    if a.profile.sha256 != sha256_hex(ctx.profile_bytes) {
        errors.push(err(
            codes::PROFILE_DIGEST_MISMATCH,
            "profile.sha256",
            "differs from the profile file",
        ));
    }
    if a.profile.id != profile.profile_id {
        errors.push(err(
            codes::PROFILE_ID_MISMATCH,
            "profile.id",
            "differs from the profile file",
        ));
    }
    if a.profile.version != profile.version {
        errors.push(err(
            codes::PROFILE_VERSION_MISMATCH,
            "profile.version",
            "differs from the profile file",
        ));
    }
    if a.evaluation.sha256 != sha256_hex(ctx.evaluation_bytes) {
        errors.push(err(
            cc::EVALUATION_DIGEST_MISMATCH,
            "evaluation.sha256",
            "differs from the evaluation file",
        ));
    }
    if evaluation.profile != a.profile {
        errors.push(err(
            cc::EVALUATION_PROFILE_MISMATCH,
            "profile",
            "the evaluation cites a different profile",
        ));
    }
    if a.decision != profile.decision {
        errors.push(err(
            cc::DECISION_MISMATCH,
            "decision",
            "differs from the profile's decision",
        ));
    }

    // Rule 2: coverage.
    let evaluated: BTreeMap<&str, &EvaluatedRequirement> = evaluation
        .requirements
        .iter()
        .map(|r| (r.id.as_str(), r))
        .collect();
    let mut entries: BTreeMap<&str, &AssessedRequirement> = BTreeMap::new();
    for (i, entry) in a.requirements.iter().enumerate() {
        let at = format!("requirements[{i}]");
        if !profile.requirements.iter().any(|r| r.id == entry.id) {
            errors.push(err(cc::REQUIREMENT_UNKNOWN, format!("{at}.id"), &entry.id));
        } else if entries.insert(entry.id.as_str(), entry).is_some() {
            errors.push(err(
                cc::REQUIREMENT_DUPLICATE,
                format!("{at}.id"),
                &entry.id,
            ));
        }
    }
    for requirement in &profile.requirements {
        let Some(entry) = entries.get(requirement.id.as_str()) else {
            errors.push(err(
                cc::REQUIREMENT_MISSING,
                "requirements",
                &requirement.id,
            ));
            continue;
        };
        let at = format!("requirements[{}]", requirement.id);
        let actual = evaluated.get(requirement.id.as_str()).map(|r| r.status);
        if actual != Some(entry.evaluation_status) {
            errors.push(err(
                cc::EVALUATION_STATUS_MISMATCH,
                format!("{at}.evaluation_status"),
                "differs from the evaluation's status",
            ));
        }
        match requirement.class {
            RequirementClass::Mechanical => {
                let from = actual.unwrap_or(entry.evaluation_status);
                if severity(entry.assessed_status) < severity(from) {
                    errors.push(err(
                        cc::MECHANICAL_UPGRADE,
                        format!("{at}.assessed_status"),
                        "a mechanical status may be downgraded, never upgraded",
                    ));
                } else if severity(entry.assessed_status) > severity(from) {
                    if entry.basis != AssessmentBasis::Assessor {
                        errors.push(err(
                            cc::DOWNGRADE_BASIS_NOT_ASSESSOR,
                            format!("{at}.basis"),
                            "a downgrade has basis assessor",
                        ));
                    }
                    if entry.reason.trim().is_empty() {
                        errors.push(err(
                            cc::DOWNGRADE_REASON_MISSING,
                            format!("{at}.reason"),
                            "a downgrade states a reason",
                        ));
                    }
                    if entry.evidence.is_empty() {
                        errors.push(err(
                            cc::DOWNGRADE_EVIDENCE_MISSING,
                            format!("{at}.evidence"),
                            "a downgrade cites evidence",
                        ));
                    }
                }
            }
            RequirementClass::Attributed => {
                if entry.reason.trim().is_empty() {
                    errors.push(err(
                        cc::ATTRIBUTED_REASON_MISSING,
                        format!("{at}.reason"),
                        "an attributed status states a reason",
                    ));
                }
                if entry.evidence.is_empty() {
                    errors.push(err(
                        cc::ATTRIBUTED_EVIDENCE_MISSING,
                        format!("{at}.evidence"),
                        "an attributed status cites evidence",
                    ));
                }
            }
        }
    }

    // Rule 5: verdict consistency over every assessed status.
    let derived = if a
        .requirements
        .iter()
        .any(|r| r.assessed_status == Status::Unmet)
    {
        VerdictValue::Blocked
    } else if a
        .requirements
        .iter()
        .any(|r| r.assessed_status == Status::Indeterminate)
    {
        VerdictValue::Uncertain
    } else {
        VerdictValue::Ready
    };
    if verdict_rank(a.verdict) < verdict_rank(derived) {
        errors.push(err(
            cc::VERDICT_INCONSISTENT,
            "verdict",
            "less conservative than the verdict derived from the assessed statuses",
        ));
    } else if verdict_rank(a.verdict) > verdict_rank(derived) && a.profile_gaps.is_empty() {
        errors.push(err(
            cc::VERDICT_INCONSISTENT,
            "verdict",
            "more conservative than derived, with no profile_gaps entry naming why",
        ));
    }

    // Rule 6: sets and role.
    let implied = |status: Status| {
        let mut ids: Vec<&str> = a
            .requirements
            .iter()
            .filter(|r| r.assessed_status == status)
            .map(|r| r.id.as_str())
            .collect();
        ids.sort_unstable();
        ids
    };

    if sorted(&a.missing) != implied(Status::Unmet) {
        errors.push(err(
            cc::MISSING_SET_MISMATCH,
            "missing",
            "differs from the requirements assessed unmet",
        ));
    }
    if sorted(&a.indeterminate) != implied(Status::Indeterminate) {
        errors.push(err(
            cc::INDETERMINATE_SET_MISMATCH,
            "indeterminate",
            "differs from the requirements assessed indeterminate",
        ));
    }
    if !profile.assessor_roles.contains(&a.assessor.role) {
        errors.push(err(
            cc::ASSESSOR_ROLE_NOT_ALLOWED,
            "assessor.role",
            "not in the profile's assessor_roles",
        ));
    }
    if a.assessor.authority_ref.trim().is_empty() {
        errors.push(err(
            cc::AUTHORITY_REF_EMPTY,
            "assessor.authority_ref",
            "empty",
        ));
    }

    // Rule 7: freshness basis equals the evaluation's battery and observation fields.
    let o = &a.freshness_basis.observation;
    for (field, got, want) in [
        (
            "battery_file_sha256",
            &o.battery_file_sha256,
            &evaluation.battery.file_sha256,
        ),
        (
            "battery_sha256",
            &o.battery_sha256,
            &evaluation.battery.battery_sha256,
        ),
        (
            "observation_sha256",
            &o.observation_sha256,
            &evaluation.observation.sha256,
        ),
        (
            "started_at",
            &o.started_at,
            &evaluation.observation.started_at,
        ),
        (
            "finished_at",
            &o.finished_at,
            &evaluation.observation.finished_at,
        ),
    ] {
        if got != want {
            errors.push(err(
                cc::FRESHNESS_BASIS_MISMATCH,
                format!("freshness_basis.observation.{field}"),
                "differs from the evaluation",
            ));
        }
    }
    errors
}

/// The `purpose` of a raw assessment document when it is one of the three valid values,
/// so `check` can echo it even when the document fails the closed schema.
pub fn raw_purpose(bytes: &[u8]) -> Option<String> {
    let value: Value = serde_json::from_slice(bytes).ok()?;
    let purpose = value.get("purpose")?.as_str()?;
    ["live", "fixture", "demonstration"]
        .contains(&purpose)
        .then(|| purpose.to_string())
}

/// Derived check ids whose recorded check object differs between two battery results. An id
/// missing from the second result counts as changed (a literal reading of contract 5.4).
pub fn changed_checks(derived_ids: &[String], first: &Value, second: &Value) -> Vec<String> {
    derived_ids
        .iter()
        .filter(|id| first["checks"].get(id.as_str()) != second["checks"].get(id.as_str()))
        .cloned()
        .collect()
}
