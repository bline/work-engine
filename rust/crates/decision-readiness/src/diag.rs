//! Stable error codes and the `check` report shape (contract 2.2).

use serde::Serialize;

/// Stable error codes. A code never changes meaning; new rules add new codes.
pub mod codes {
    // Document-level
    pub const SCHEMA: &str = "schema";
    pub const SCHEMA_MISMATCH: &str = "schema_mismatch";
    // Profile
    pub const INVALID_PROFILE_ID: &str = "invalid_profile_id";
    pub const INVALID_REQUIREMENT_ID: &str = "invalid_requirement_id";
    pub const DUPLICATE_REQUIREMENT_ID: &str = "duplicate_requirement_id";
    pub const EMPTY_NAME: &str = "empty_name";
    pub const DUPLICATE_ROOT: &str = "duplicate_root";
    pub const MULTIPLE_GIT_ROOTS: &str = "multiple_git_roots";
    pub const GIT_PREDICATE_WITHOUT_GIT_ROOT: &str = "git_predicate_without_git_root";
    pub const DUPLICATE_PARAMETER: &str = "duplicate_parameter";
    pub const ENTRIES_ROOT_REQUIRED: &str = "entries_root_required";
    pub const ENTRIES_ROOT_UNEXPECTED: &str = "entries_root_unexpected";
    pub const BOUND_BY_UNEXPECTED_KIND: &str = "bound_by_unexpected_kind";
    pub const BOUND_BY_INVALID_TARGET: &str = "bound_by_invalid_target";
    pub const UNKNOWN_ROOT: &str = "unknown_root";
    pub const UNKNOWN_PARAMETER: &str = "unknown_parameter";
    pub const PARAM_KIND_MISMATCH: &str = "param_kind_mismatch";
    pub const MECHANICAL_WITHOUT_PREDICATES: &str = "mechanical_without_predicates";
    pub const ATTRIBUTED_WITH_PREDICATES: &str = "attributed_with_predicates";
    pub const MECHANICAL_WITH_EVIDENCE_FROM: &str = "mechanical_with_evidence_from";
    pub const UNKNOWN_EVIDENCE_REF: &str = "unknown_evidence_ref";
    pub const INVALID_RELATIVE_PATH: &str = "invalid_relative_path";
    pub const INVALID_SHA256: &str = "invalid_sha256";
    pub const FLOAT_OR_NULL_EXPECTED: &str = "float_or_null_expected";
    pub const EACH_PLACEHOLDER_MISMATCH: &str = "each_placeholder_mismatch";
    pub const EMPTY_KEY: &str = "empty_key";
    pub const EMPTY_FILES: &str = "empty_files";
    pub const GIT_ARGS_EMPTY: &str = "git_args_empty";
    pub const GIT_SUBCOMMAND_NOT_LITERAL: &str = "git_subcommand_not_literal";
    pub const GIT_SUBCOMMAND_FORBIDDEN: &str = "git_subcommand_forbidden";
    pub const GIT_ARGUMENT_FORBIDDEN: &str = "git_argument_forbidden";
    // Subject
    pub const PROFILE_ID_MISMATCH: &str = "profile_id_mismatch";
    pub const PROFILE_VERSION_MISMATCH: &str = "profile_version_mismatch";
    pub const PROFILE_DIGEST_MISMATCH: &str = "profile_digest_mismatch";
    pub const MISSING_ROOT: &str = "missing_root";
    pub const UNKNOWN_ROOT_PATH: &str = "unknown_root_path";
    pub const ROOT_NOT_NORMALISED: &str = "root_not_normalised";
    pub const MISSING_VALUE: &str = "missing_value";
    pub const UNKNOWN_VALUE: &str = "unknown_value";
    pub const VALUE_TYPE_MISMATCH: &str = "value_type_mismatch";
    pub const EMPTY_LIST_VALUE: &str = "empty_list_value";
    pub const MANIFEST_DIGEST_MISMATCH: &str = "manifest_digest_mismatch";
    pub const MANIFEST_INVALID: &str = "manifest_invalid";
    pub const EMPTY_MANIFEST: &str = "empty_manifest";
    pub const BOUND_BY_PATH_ABSENT: &str = "bound_by_path_absent";
    pub const BOUND_BY_ROOT_MISMATCH: &str = "bound_by_root_mismatch";
    // Multi-profile plan
    pub const DUPLICATE_PROFILE: &str = "duplicate_profile";
    pub const ROOT_PATH_CONFLICT: &str = "root_path_conflict";
    pub const MULTIPLE_GIT_ROOT_PATHS: &str = "multiple_git_root_paths";
    pub const INTERNAL: &str = "internal_inconsistency";
    // Observation
    pub const INVALID_ABSOLUTE_PATH: &str = "invalid_absolute_path";
    pub const BATTERY_EXIT_INVALID: &str = "battery_exit_invalid";
    pub const INVALID_TIMESTAMP: &str = "invalid_timestamp";
    pub const TIME_ORDER: &str = "time_order";
}

/// One violated rule: a stable `code`, where it is (`at`), and a human message.
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

    /// The same error, with `prefix` added to its location.
    pub fn prefixed(mut self, prefix: &str) -> Self {
        self.at = if self.at.is_empty() {
            prefix.to_string()
        } else {
            format!("{prefix}.{}", self.at)
        };
        self
    }
}

/// `{valid, kind, purpose?, errors[]}` (contract 2.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CheckReport {
    pub valid: bool,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    pub errors: Vec<ErrorItem>,
}

impl CheckReport {
    pub fn new(kind: &str, errors: Vec<ErrorItem>) -> Self {
        CheckReport {
            valid: errors.is_empty(),
            kind: kind.to_string(),
            purpose: None,
            errors,
        }
    }
}
