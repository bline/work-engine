//! Stable error codes, the `check` report shape (contract S8) and the failure type.

use serde::Serialize;
use thiserror::Error;

/// Stable error codes. A code never changes meaning; new rules add new codes.
pub mod codes {
    // Shared by every record (contract S8).
    pub const SCHEMA: &str = "schema";
    pub const SCHEMA_MISMATCH: &str = "schema_mismatch";
    pub const ID_PATTERN: &str = "id_pattern";
    pub const EMPTY_STRING: &str = "empty_string";
    pub const TIME_FORMAT: &str = "time_format";
    pub const PATH_NOT_NORMALISED: &str = "path_not_normalised";
    pub const SHA256_PATTERN: &str = "sha256_pattern";
    pub const DIGEST_MISMATCH: &str = "digest_mismatch";
    pub const POINTER_UNRESOLVED: &str = "pointer_unresolved";
    pub const UPSTREAM_CLASS_MISMATCH: &str = "upstream_class_mismatch";
    pub const AUTHORITY_ASSERTED: &str = "authority_asserted";
    pub const DUPLICATE_PROPOSAL_ID: &str = "duplicate_proposal_id";
    pub const DUPLICATE_MEASURE_ID: &str = "duplicate_measure_id";
    pub const LIMITATION_MISSING: &str = "limitation_missing";
    pub const PATH_NOT_ABSOLUTE: &str = "path_not_absolute";
    // Estimate set (contract 2.4).
    pub const PROPOSAL_ID_MISMATCH: &str = "proposal_id_mismatch";
    pub const CUTOFF_AFTER_ISSUE: &str = "cutoff_after_issue";
    pub const CLAIM_PROJECTION_PRESENCE: &str = "claim_projection_presence";
    pub const SCALE_INVALID: &str = "scale_invalid";
    pub const VALUE_SCALE_MISMATCH: &str = "value_scale_mismatch";
    pub const ESTIMATE_CUTOFF_AFTER_SET_CUTOFF: &str = "estimate_cutoff_after_set_cutoff";
    pub const INTERVAL_OUT_OF_ORDER: &str = "interval_out_of_order";
    pub const BASIS_REQUIRED: &str = "basis_required";
    pub const COMPETING_INSUFFICIENT: &str = "competing_insufficient";
    pub const BASIS_UNRESOLVED: &str = "basis_unresolved";
    pub const BASIS_WRONG_PROFILE: &str = "basis_wrong_profile";
    // Named by the builder (contract S8, last sentence); see `public_api` in the result.
    /// `estimates` is empty (a set holds at least one estimate).
    pub const ESTIMATES_EMPTY: &str = "estimates_empty";
    /// An ordinal value, interval end or baseline names a label not in the scale's `levels`.
    pub const LEVEL_UNKNOWN: &str = "level_unknown";
    // Comparison contract (contract 3.6).
    pub const VERSION_INVALID: &str = "version_invalid";
    pub const CONTRACT_PROPOSALS_INSUFFICIENT: &str = "contract_proposals_insufficient";
    pub const STALE_BEFORE_AFTER_CUTOFF: &str = "stale_before_after_cutoff";
    /// Named by the builder: `measures` is empty (a contract includes at least one measure).
    pub const MEASURES_EMPTY: &str = "measures_empty";
}

/// One violated rule: a stable `code`, where it is (`at`, a dotted path), and a message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ErrorItem {
    pub code: String,
    pub at: String,
    pub message: String,
}

impl ErrorItem {
    pub fn new(code: &str, at: impl Into<String>, message: impl Into<String>) -> Self {
        ErrorItem {
            code: code.to_string(),
            at: at.into(),
            message: message.into(),
        }
    }
}

/// `{valid, kind, errors[], notes[]?}` (contract S8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckReport {
    pub valid: bool,
    pub kind: String,
    pub errors: Vec<ErrorItem>,
    /// Limitations of the check worth reporting alongside the verdict.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl CheckReport {
    pub fn new(kind: &str, errors: Vec<ErrorItem>, notes: Vec<String>) -> Self {
        CheckReport {
            valid: errors.is_empty(),
            kind: kind.to_string(),
            errors,
            notes,
        }
    }
}

pub const EXIT_OK: i32 = 0;
pub const EXIT_INVALID: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_BINDING: i32 = 3;

/// Why a command stopped without a report: exit 2 (usage, IO, parse, refused input) or
/// exit 3 (a binding or digest refusal by a deriving command or `--rederive`).
#[derive(Debug, Error)]
pub enum Failure {
    #[error("{0}")]
    Usage(String),
    #[error("cannot read {path}: {message}")]
    Io { path: String, message: String },
    #[error("malformed JSON: {0}")]
    Parse(String),
    /// An input refused before any work, such as `path_not_absolute` (exit 2).
    #[error("{message}")]
    Refused {
        message: String,
        errors: Vec<ErrorItem>,
    },
    /// A binding or digest refusal (exit 3); nothing is written.
    #[error("{message}")]
    Binding {
        message: String,
        errors: Vec<ErrorItem>,
    },
}

impl Failure {
    pub fn exit_code(&self) -> i32 {
        match self {
            Failure::Binding { .. } => EXIT_BINDING,
            _ => EXIT_USAGE,
        }
    }

    pub fn errors(&self) -> &[ErrorItem] {
        match self {
            Failure::Refused { errors, .. } | Failure::Binding { errors, .. } => errors,
            _ => &[],
        }
    }
}
