//! Validation of profile, subject and observation documents.
//!
//! Every function reports every violated rule with a stable code (see
//! [`crate::diag::codes`]) instead of stopping at the first.

use crate::diag::{ErrorItem, codes};
use crate::digest::{is_sha256_hex, sha256_hex};
use crate::model::*;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// Git subcommands the evidence battery accepts (`B:17-20`).
pub const READ_ONLY_GIT: [&str; 12] = [
    "rev-parse",
    "status",
    "ls-files",
    "diff",
    "log",
    "show",
    "cat-file",
    "merge-base",
    "describe",
    "rev-list",
    "for-each-ref",
    "ls-tree",
];

/// Argument prefixes the battery refuses (`B:21`: `^(--output|--exec|--upload-pack|-c)`).
const FORBIDDEN_GIT_PREFIXES: [&str; 4] = ["--output", "--exec", "--upload-pack", "-c"];

/// The `{each}` placeholder in a JSON key.
pub const EACH_PLACEHOLDER: &str = "{each}";

/// Reads a file by absolute path; the only filesystem access validation has.
pub type ReadFile<'a> = &'a dyn Fn(&str) -> std::io::Result<Vec<u8>>;

#[derive(Debug, Error)]
#[error("cannot read {path}: {source}")]
pub struct ReadError {
    pub path: String,
    #[source]
    pub source: std::io::Error,
}

/// Why a document could not be turned into its type.
#[derive(Debug, Error)]
pub enum DocError {
    /// Not JSON at all: a usage-class failure (exit 2).
    #[error("malformed JSON: {0}")]
    Syntax(String),
    /// JSON that violates the closed schema: an unknown field, a missing field,
    /// a wrong type. `check` reports it as an invalid document (exit 1).
    #[error("{0}")]
    Schema(String),
}

/// Parse `bytes` as `T`. Duplicate keys in a struct are a schema error.
pub fn parse_document<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, DocError> {
    serde_json::from_slice::<T>(bytes).map_err(|error| match error.classify() {
        serde_json::error::Category::Data => DocError::Schema(error.to_string()),
        _ => DocError::Syntax(error.to_string()),
    })
}

// ---------------------------------------------------------------------------
// Lexical helpers
// ---------------------------------------------------------------------------

/// Battery identity pattern `[A-Za-z0-9][A-Za-z0-9._-]*` (`B:22`), optionally without the dot.
fn matches_identity(text: &str, allow_dot: bool) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphanumeric() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || (allow_dot && c == '.'))
}

pub fn is_profile_id(text: &str) -> bool {
    matches_identity(text, true)
}

/// Requirement ids match `[A-Za-z0-9][A-Za-z0-9_-]*`: no dot, so a check id splits unambiguously.
pub fn is_requirement_id(text: &str) -> bool {
    matches_identity(text, false)
}

fn bad_component(component: &str) -> bool {
    component.is_empty() || component == "." || component == ".."
}

/// Absolute and normalised: begins with `/`, no `.`, `..` or empty component, no trailing `/`.
pub fn is_normalised_absolute(text: &str) -> bool {
    match text.strip_prefix('/') {
        Some(rest) => !rest.is_empty() && !rest.split('/').any(bad_component),
        None => false,
    }
}

/// Relative and normalised: non-empty, not absolute, no `.`, `..` or empty component.
pub fn is_normalised_relative(text: &str) -> bool {
    !text.is_empty() && !text.starts_with('/') && !text.split('/').any(bad_component)
}

/// `YYYY-MM-DDTHH:MM:SSZ`, a real UTC calendar time.
pub fn is_utc_timestamp(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 20 {
        return false;
    }
    let shape = b"dddd-dd-ddTdd:dd:ddZ";
    for (byte, want) in bytes.iter().zip(shape.iter()) {
        let ok = if *want == b'd' {
            byte.is_ascii_digit()
        } else {
            byte == want
        };
        if !ok {
            return false;
        }
    }
    let number = |from: usize, to: usize| -> u32 { text[from..to].parse().unwrap_or(u32::MAX) };
    let (year, month, day) = (number(0, 4), number(5, 7), number(8, 10));
    let (hour, minute, second) = (number(11, 13), number(14, 16), number(17, 19));
    if !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if leap => 29,
        _ => 28,
    };
    (1..=days).contains(&day)
}

fn has_float_or_null(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Number(number) => number.is_f64(),
        Value::Array(items) => items.iter().any(has_float_or_null),
        Value::Object(map) => map.values().any(has_float_or_null),
        Value::Bool(_) | Value::String(_) => false,
    }
}

/// Checks a fully expanded `git` token list against the battery's rules and the
/// contract's plan-time rules (3.4 rule 3).
pub(crate) fn git_token_errors(tokens: &[String], at: &str, errors: &mut Vec<ErrorItem>) {
    let mut after_separator = false;
    for (position, token) in tokens.iter().enumerate() {
        if position == 0 {
            if !READ_ONLY_GIT.contains(&token.as_str()) {
                errors.push(ErrorItem::new(
                    codes::GIT_SUBCOMMAND_FORBIDDEN,
                    at,
                    format!("git {token} is not a declared read-only subcommand"),
                ));
            }
        } else if after_separator && token.starts_with('-') {
            errors.push(ErrorItem::new(
                codes::GIT_ARGUMENT_FORBIDDEN,
                at,
                format!("argument {token:?} begins with '-' after a -- separator"),
            ));
        }
        if FORBIDDEN_GIT_PREFIXES
            .iter()
            .any(|prefix| token.starts_with(prefix))
        {
            errors.push(ErrorItem::new(
                codes::GIT_ARGUMENT_FORBIDDEN,
                at,
                format!("git argument not permitted here: {token}"),
            ));
        }
        if token == "--" {
            after_separator = true;
        }
    }
}

// ---------------------------------------------------------------------------
// Profile
// ---------------------------------------------------------------------------

struct ProfileContext<'a> {
    roots: BTreeSet<&'a str>,
    params: BTreeMap<&'a str, &'a ParamDecl>,
    has_git_root: bool,
}

impl ProfileContext<'_> {
    fn param(&self, name: &str, want: &[ParamKind], at: &str, errors: &mut Vec<ErrorItem>) {
        match self.params.get(name) {
            None => errors.push(ErrorItem::new(
                codes::UNKNOWN_PARAMETER,
                at,
                format!("no parameter named {name:?}"),
            )),
            Some(decl) if !want.contains(&decl.kind) => errors.push(ErrorItem::new(
                codes::PARAM_KIND_MISMATCH,
                at,
                format!(
                    "parameter {name:?} has kind {:?}, expected one of {want:?}",
                    decl.kind
                ),
            )),
            Some(_) => {}
        }
    }

    fn path_ref(&self, reference: &PathRef, at: &str, errors: &mut Vec<ErrorItem>) {
        match reference {
            PathRef::Param(param) => {
                self.param(&param.param, &[ParamKind::Path], at, errors);
            }
            PathRef::Literal(literal) => {
                if !self.roots.contains(literal.root.as_str()) {
                    errors.push(ErrorItem::new(
                        codes::UNKNOWN_ROOT,
                        at,
                        format!("no root named {:?}", literal.root),
                    ));
                }
                if !is_normalised_relative(&literal.path) {
                    errors.push(ErrorItem::new(
                        codes::INVALID_RELATIVE_PATH,
                        at,
                        format!("path {:?} is not a normalised relative path", literal.path),
                    ));
                }
            }
        }
    }

    fn key_and_each(
        &self,
        key: &str,
        each: &Option<ParamRef>,
        at: &str,
        errors: &mut Vec<ErrorItem>,
    ) {
        if key.is_empty() {
            errors.push(ErrorItem::new(codes::EMPTY_KEY, at, "key is empty"));
        }
        let has_placeholder = key.contains(EACH_PLACEHOLDER);
        if has_placeholder != each.is_some() {
            errors.push(ErrorItem::new(
                codes::EACH_PLACEHOLDER_MISMATCH,
                at,
                "a key contains {each} exactly when the predicate declares `each`",
            ));
        }
        if let Some(each) = each {
            self.param(&each.param, &[ParamKind::List], at, errors);
        }
    }

    fn git_args(&self, args: &[StrVal], at: &str, errors: &mut Vec<ErrorItem>) {
        if args.is_empty() {
            errors.push(ErrorItem::new(
                codes::GIT_ARGS_EMPTY,
                at,
                "git args are empty",
            ));
            return;
        }
        if matches!(args[0], StrVal::Param(_)) {
            errors.push(ErrorItem::new(
                codes::GIT_SUBCOMMAND_NOT_LITERAL,
                at,
                "the git subcommand must be a literal so plan can check it",
            ));
        }
        let mut tokens = Vec::with_capacity(args.len());
        for (position, arg) in args.iter().enumerate() {
            match arg {
                StrVal::Literal(text) => tokens.push(text.clone()),
                StrVal::Param(param) => {
                    // Only a list parameter expands in place inside an array.
                    self.param(&param.param, &[ParamKind::List], at, errors);
                    // A stand-in that passes the token rules; a parameter in
                    // position 0 was already reported as not literal.
                    tokens.push(
                        if position == 0 {
                            "rev-parse"
                        } else {
                            "<param>"
                        }
                        .to_string(),
                    );
                }
            }
        }
        git_token_errors(&tokens, at, errors);
    }

    fn str_val(&self, value: &StrVal, at: &str, errors: &mut Vec<ErrorItem>) {
        if let StrVal::Param(param) = value {
            self.param(&param.param, &[ParamKind::String], at, errors);
        }
    }

    fn predicate(&self, predicate: &Predicate, at: &str, errors: &mut Vec<ErrorItem>) {
        if predicate.is_git() && !self.has_git_root {
            errors.push(ErrorItem::new(
                codes::GIT_PREDICATE_WITHOUT_GIT_ROOT,
                at,
                "a git_* predicate needs a root declared with git: true",
            ));
        }
        match predicate {
            Predicate::FileSha256(p) => {
                self.path_ref(&p.file, &format!("{at}.file"), errors);
                self.str_val(&p.expected, &format!("{at}.expected"), errors);
                if let StrVal::Literal(text) = &p.expected
                    && !is_sha256_hex(text)
                {
                    errors.push(ErrorItem::new(
                        codes::INVALID_SHA256,
                        format!("{at}.expected"),
                        "expected is not 64 lowercase hex characters",
                    ));
                }
            }
            Predicate::ManifestMatch(p) => {
                self.param(
                    &p.manifest.param,
                    &[ParamKind::Manifest],
                    &format!("{at}.manifest"),
                    errors,
                );
            }
            Predicate::JsonEquals(p) => {
                self.path_ref(&p.file, &format!("{at}.file"), errors);
                self.key_and_each(&p.key, &p.each, at, errors);
                match &p.expected {
                    Expected::Param(param) => self.param(
                        &param.param,
                        &[ParamKind::String],
                        &format!("{at}.expected"),
                        errors,
                    ),
                    Expected::Literal(value) => {
                        if has_float_or_null(value) {
                            errors.push(ErrorItem::new(
                                codes::FLOAT_OR_NULL_EXPECTED,
                                format!("{at}.expected"),
                                "expected may not be, or contain, a float or null",
                            ));
                        }
                    }
                }
            }
            Predicate::JsonCompare(p) => {
                self.path_ref(&p.file, &format!("{at}.file"), errors);
                self.key_and_each(&p.key, &p.each, at, errors);
            }
            Predicate::JsonPresent(p) => {
                self.path_ref(&p.file, &format!("{at}.file"), errors);
                self.key_and_each(&p.key, &p.each, at, errors);
            }
            Predicate::GrepCount(p) => match &p.files {
                FilesSpec::Param(param) => self.param(
                    &param.param,
                    &[ParamKind::PathList],
                    &format!("{at}.files"),
                    errors,
                ),
                FilesSpec::List(list) => {
                    if list.is_empty() {
                        errors.push(ErrorItem::new(
                            codes::EMPTY_FILES,
                            format!("{at}.files"),
                            "files is an empty array",
                        ));
                    }
                    for (n, reference) in list.iter().enumerate() {
                        self.path_ref(reference, &format!("{at}.files[{n}]"), errors);
                    }
                }
            },
            Predicate::GitOutputEquals(p) => {
                self.git_args(&p.args, &format!("{at}.args"), errors);
                self.str_val(&p.expected, &format!("{at}.expected"), errors);
            }
            Predicate::GitOutputEmpty(p) => {
                self.git_args(&p.args, &format!("{at}.args"), errors);
            }
        }
    }
}

/// Validate a parsed profile: every rule of contract 3.1, 3.5 and 3.7 that needs no subject.
pub fn validate_profile(profile: &Profile) -> Vec<ErrorItem> {
    let mut errors = Vec::new();
    if profile.schema != PROFILE_SCHEMA {
        errors.push(ErrorItem::new(
            codes::SCHEMA_MISMATCH,
            "schema",
            format!("schema must be {PROFILE_SCHEMA:?}"),
        ));
    }
    if !is_profile_id(&profile.profile_id) {
        errors.push(ErrorItem::new(
            codes::INVALID_PROFILE_ID,
            "profile_id",
            "profile_id must match [A-Za-z0-9][A-Za-z0-9._-]*",
        ));
    }

    // Roots
    let mut root_names = BTreeSet::new();
    let mut git_roots = 0;
    for (n, root) in profile.roots.iter().enumerate() {
        let at = format!("roots[{n}]");
        if root.name.is_empty() {
            errors.push(ErrorItem::new(codes::EMPTY_NAME, &at, "root name is empty"));
        }
        if !root_names.insert(root.name.as_str()) {
            errors.push(ErrorItem::new(
                codes::DUPLICATE_ROOT,
                &at,
                format!("root {:?} is declared twice", root.name),
            ));
        }
        if root.git {
            git_roots += 1;
        }
    }
    if git_roots > 1 {
        errors.push(ErrorItem::new(
            codes::MULTIPLE_GIT_ROOTS,
            "roots",
            "at most one root may have git: true",
        ));
    }

    // Parameters
    let mut params: BTreeMap<&str, &ParamDecl> = BTreeMap::new();
    for (n, param) in profile.parameters.iter().enumerate() {
        let at = format!("parameters[{n}]");
        if param.name.is_empty() {
            errors.push(ErrorItem::new(
                codes::EMPTY_NAME,
                &at,
                "parameter name is empty",
            ));
        }
        if params.insert(param.name.as_str(), param).is_some() {
            errors.push(ErrorItem::new(
                codes::DUPLICATE_PARAMETER,
                &at,
                format!("parameter {:?} is declared twice", param.name),
            ));
        }
    }
    for (n, param) in profile.parameters.iter().enumerate() {
        let at = format!("parameters[{n}]");
        match (param.kind, &param.entries_root) {
            (ParamKind::Manifest, None) => errors.push(ErrorItem::new(
                codes::ENTRIES_ROOT_REQUIRED,
                &at,
                "a manifest parameter names the root its entries resolve against",
            )),
            (ParamKind::Manifest, Some(root)) if !root_names.contains(root.as_str()) => {
                errors.push(ErrorItem::new(
                    codes::UNKNOWN_ROOT,
                    format!("{at}.entries_root"),
                    format!("no root named {root:?}"),
                ));
            }
            (kind, Some(_)) if kind != ParamKind::Manifest => errors.push(ErrorItem::new(
                codes::ENTRIES_ROOT_UNEXPECTED,
                &at,
                "entries_root belongs to manifest parameters only",
            )),
            _ => {}
        }
        if let Some(target) = &param.bound_by {
            if !matches!(param.kind, ParamKind::Path | ParamKind::PathList) {
                errors.push(ErrorItem::new(
                    codes::BOUND_BY_UNEXPECTED_KIND,
                    &at,
                    "bound_by belongs to path and path_list parameters only",
                ));
            }
            match params.get(target.as_str()) {
                Some(decl) if decl.kind == ParamKind::Manifest => {}
                _ => errors.push(ErrorItem::new(
                    codes::BOUND_BY_INVALID_TARGET,
                    format!("{at}.bound_by"),
                    format!("{target:?} is not a manifest parameter"),
                )),
            }
        }
    }

    // Requirements
    let context = ProfileContext {
        roots: root_names,
        params,
        has_git_root: git_roots >= 1,
    };
    let mut seen_ids = BTreeSet::new();
    let mechanical_ids: BTreeSet<&str> = profile
        .requirements
        .iter()
        .filter(|r| r.class == RequirementClass::Mechanical)
        .map(|r| r.id.as_str())
        .collect();
    for (n, requirement) in profile.requirements.iter().enumerate() {
        let at = format!("requirements[{n}]");
        if !is_requirement_id(&requirement.id) {
            errors.push(ErrorItem::new(
                codes::INVALID_REQUIREMENT_ID,
                format!("{at}.id"),
                format!(
                    "requirement id {:?} must match [A-Za-z0-9][A-Za-z0-9_-]* (no dot)",
                    requirement.id
                ),
            ));
        }
        if !seen_ids.insert(requirement.id.as_str()) {
            errors.push(ErrorItem::new(
                codes::DUPLICATE_REQUIREMENT_ID,
                format!("{at}.id"),
                format!("requirement id {:?} is used twice", requirement.id),
            ));
        }
        match requirement.class {
            RequirementClass::Mechanical => {
                if requirement.predicates.is_empty() {
                    errors.push(ErrorItem::new(
                        codes::MECHANICAL_WITHOUT_PREDICATES,
                        &at,
                        "a mechanical requirement has one or more predicates",
                    ));
                }
                if !requirement.evidence_from.is_empty() {
                    errors.push(ErrorItem::new(
                        codes::MECHANICAL_WITH_EVIDENCE_FROM,
                        &at,
                        "evidence_from belongs to attributed requirements",
                    ));
                }
                for (i, predicate) in requirement.predicates.iter().enumerate() {
                    context.predicate(predicate, &format!("{at}.predicates[{i}]"), &mut errors);
                }
            }
            RequirementClass::Attributed => {
                if !requirement.predicates.is_empty() {
                    errors.push(ErrorItem::new(
                        codes::ATTRIBUTED_WITH_PREDICATES,
                        &at,
                        "an attributed requirement has no predicates",
                    ));
                }
                for (i, id) in requirement.evidence_from.iter().enumerate() {
                    if !mechanical_ids.contains(id.as_str()) {
                        errors.push(ErrorItem::new(
                            codes::UNKNOWN_EVIDENCE_REF,
                            format!("{at}.evidence_from[{i}]"),
                            format!("{id:?} is not a mechanical requirement of this profile"),
                        ));
                    }
                }
            }
        }
    }
    errors
}

// ---------------------------------------------------------------------------
// Subject
// ---------------------------------------------------------------------------

/// A subject value typed by its parameter's kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedValue {
    Str(String),
    List(Vec<String>),
    Path(PathValue),
    PathList(Vec<PathValue>),
    Manifest(ManifestValue),
}

/// A subject that passed validation, with its manifests read and verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedSubject {
    pub roots: BTreeMap<String, String>,
    pub values: BTreeMap<String, TypedValue>,
    /// Entries per manifest parameter, in manifest order.
    pub manifests: BTreeMap<String, Vec<ManifestEntry>>,
}

#[derive(Debug)]
pub struct SubjectOutcome {
    pub errors: Vec<ErrorItem>,
    /// Present exactly when `errors` is empty.
    pub typed: Option<TypedSubject>,
}

/// `<root path>/<relative path>`: the absolute path every derived check uses (contract 3.4 rule 1).
pub fn absolute_path(root_path: &str, relative: &str) -> String {
    format!("{root_path}/{relative}")
}

fn type_value(
    kind: ParamKind,
    value: &Value,
    at: &str,
    errors: &mut Vec<ErrorItem>,
) -> Option<TypedValue> {
    let mismatch = |message: String, errors: &mut Vec<ErrorItem>| {
        errors.push(ErrorItem::new(codes::VALUE_TYPE_MISMATCH, at, message));
    };
    match kind {
        ParamKind::String => match value {
            Value::String(text) => Some(TypedValue::Str(text.clone())),
            _ => {
                mismatch("a string parameter takes a JSON string".to_string(), errors);
                None
            }
        },
        ParamKind::List => match serde_json::from_value::<Vec<String>>(value.clone()) {
            Ok(items) => {
                if items.is_empty() {
                    errors.push(ErrorItem::new(
                        codes::EMPTY_LIST_VALUE,
                        at,
                        "a list value may not be empty",
                    ));
                }
                Some(TypedValue::List(items))
            }
            Err(error) => {
                mismatch(
                    format!("a list parameter takes an array of strings: {error}"),
                    errors,
                );
                None
            }
        },
        ParamKind::Path => match serde_json::from_value::<PathValue>(value.clone()) {
            Ok(path) => Some(TypedValue::Path(path)),
            Err(error) => {
                mismatch(
                    format!("a path parameter takes {{root, path}}: {error}"),
                    errors,
                );
                None
            }
        },
        ParamKind::PathList => match serde_json::from_value::<Vec<PathValue>>(value.clone()) {
            Ok(items) => {
                if items.is_empty() {
                    errors.push(ErrorItem::new(
                        codes::EMPTY_LIST_VALUE,
                        at,
                        "a path_list value may not be empty",
                    ));
                }
                Some(TypedValue::PathList(items))
            }
            Err(error) => {
                mismatch(
                    format!("a path_list parameter takes an array of {{root, path}}: {error}"),
                    errors,
                );
                None
            }
        },
        ParamKind::Manifest => match serde_json::from_value::<ManifestValue>(value.clone()) {
            Ok(manifest) => Some(TypedValue::Manifest(manifest)),
            Err(error) => {
                mismatch(
                    format!("a manifest parameter takes {{root, path, sha256}}: {error}"),
                    errors,
                );
                None
            }
        },
    }
}

fn check_path_value(
    value: &PathValue,
    profile_roots: &BTreeSet<&str>,
    at: &str,
    errors: &mut Vec<ErrorItem>,
) {
    if !profile_roots.contains(value.root.as_str()) {
        errors.push(ErrorItem::new(
            codes::UNKNOWN_ROOT,
            at,
            format!("no root named {:?}", value.root),
        ));
    }
    if !is_normalised_relative(&value.path) {
        errors.push(ErrorItem::new(
            codes::INVALID_RELATIVE_PATH,
            at,
            format!("path {:?} is not a normalised relative path", value.path),
        ));
    }
}

fn load_manifest(
    value: &ManifestValue,
    roots: &BTreeMap<String, String>,
    at: &str,
    read: ReadFile<'_>,
    errors: &mut Vec<ErrorItem>,
) -> Result<Option<Vec<ManifestEntry>>, ReadError> {
    let Some(root_path) = roots.get(&value.root) else {
        return Ok(None);
    };
    if !is_normalised_absolute(root_path) || !is_normalised_relative(&value.path) {
        return Ok(None);
    }
    let path = absolute_path(root_path, &value.path);
    let bytes = read(&path).map_err(|source| ReadError {
        path: path.clone(),
        source,
    })?;
    let actual = sha256_hex(&bytes);
    if actual != value.sha256 {
        errors.push(ErrorItem::new(
            codes::MANIFEST_DIGEST_MISMATCH,
            at,
            format!(
                "manifest {path} has sha256 {actual}, the subject records {}",
                value.sha256
            ),
        ));
        return Ok(None);
    }
    let entries = match serde_json::from_slice::<Vec<ManifestEntry>>(&bytes) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(ErrorItem::new(
                codes::MANIFEST_INVALID,
                at,
                format!("manifest is not an array of objects with path and sha256: {error}"),
            ));
            return Ok(None);
        }
    };
    if entries.is_empty() {
        errors.push(ErrorItem::new(
            codes::EMPTY_MANIFEST,
            at,
            "a manifest with zero entries is refused",
        ));
        return Ok(None);
    }
    let mut seen = BTreeSet::new();
    let before = errors.len();
    for (n, entry) in entries.iter().enumerate() {
        if !is_normalised_relative(&entry.path) {
            errors.push(ErrorItem::new(
                codes::MANIFEST_INVALID,
                format!("{at}.entries[{n}]"),
                format!(
                    "entry path {:?} is not a normalised relative path",
                    entry.path
                ),
            ));
        }
        if !is_sha256_hex(&entry.sha256) {
            errors.push(ErrorItem::new(
                codes::MANIFEST_INVALID,
                format!("{at}.entries[{n}]"),
                "entry sha256 is not 64 lowercase hex characters",
            ));
        }
        if !seen.insert(entry.path.as_str()) {
            errors.push(ErrorItem::new(
                codes::MANIFEST_INVALID,
                format!("{at}.entries[{n}]"),
                format!("entry path {:?} appears twice", entry.path),
            ));
        }
    }
    Ok(if errors.len() == before {
        Some(entries)
    } else {
        None
    })
}

/// Expand a `git` args array with typed values; `None` when a value is missing or mistyped.
pub(crate) fn expand_git_args(
    args: &[StrVal],
    values: &BTreeMap<String, TypedValue>,
) -> Option<Vec<String>> {
    let mut tokens = Vec::new();
    for arg in args {
        match arg {
            StrVal::Literal(text) => tokens.push(text.clone()),
            StrVal::Param(param) => match values.get(&param.param)? {
                TypedValue::List(items) => tokens.extend(items.iter().cloned()),
                _ => return None,
            },
        }
    }
    Some(tokens)
}

/// Validate a parsed subject against its parsed profile.
///
/// `profile_sha256` is the sha256 of the profile's file bytes. `read` supplies the
/// bytes of the manifest files the subject names; an IO failure is a [`ReadError`],
/// not a rule violation. Every other problem is reported in the outcome.
pub fn validate_subject(
    profile: &Profile,
    profile_sha256: &str,
    subject: &Subject,
    read: ReadFile<'_>,
) -> Result<SubjectOutcome, ReadError> {
    let mut errors = Vec::new();
    if subject.schema != SUBJECT_SCHEMA {
        errors.push(ErrorItem::new(
            codes::SCHEMA_MISMATCH,
            "schema",
            format!("schema must be {SUBJECT_SCHEMA:?}"),
        ));
    }
    if subject.profile.id != profile.profile_id {
        errors.push(ErrorItem::new(
            codes::PROFILE_ID_MISMATCH,
            "profile.id",
            format!(
                "subject names {:?}, profile is {:?}",
                subject.profile.id, profile.profile_id
            ),
        ));
    }
    if subject.profile.version != profile.version {
        errors.push(ErrorItem::new(
            codes::PROFILE_VERSION_MISMATCH,
            "profile.version",
            format!(
                "subject names version {}, profile is {}",
                subject.profile.version, profile.version
            ),
        ));
    }
    if subject.profile.sha256 != profile_sha256 {
        errors.push(ErrorItem::new(
            codes::PROFILE_DIGEST_MISMATCH,
            "profile.sha256",
            format!(
                "subject records {}, the profile file has {profile_sha256}",
                subject.profile.sha256
            ),
        ));
    }

    // Roots map exactly the profile's root names.
    let profile_roots: BTreeSet<&str> = profile.roots.iter().map(|r| r.name.as_str()).collect();
    for name in &profile_roots {
        if !subject.roots.contains_key(*name) {
            errors.push(ErrorItem::new(
                codes::MISSING_ROOT,
                format!("roots.{name}"),
                "the subject maps no path for this profile root",
            ));
        }
    }
    for (name, path) in &subject.roots {
        let at = format!("roots.{name}");
        if !profile_roots.contains(name.as_str()) {
            errors.push(ErrorItem::new(
                codes::UNKNOWN_ROOT,
                &at,
                "the profile declares no root with this name",
            ));
        }
        if !is_normalised_absolute(path) {
            errors.push(ErrorItem::new(
                codes::ROOT_NOT_NORMALISED,
                &at,
                format!("{path:?} is not an absolute normalised path (no . or .. component, no trailing /)"),
            ));
        }
    }

    // Values: exactly one per parameter, typed by kind.
    let params: BTreeMap<&str, &ParamDecl> = profile
        .parameters
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();
    for name in subject.values.keys() {
        if !params.contains_key(name.as_str()) {
            errors.push(ErrorItem::new(
                codes::UNKNOWN_VALUE,
                format!("values.{name}"),
                "the profile declares no parameter with this name",
            ));
        }
    }
    let mut typed_values: BTreeMap<String, TypedValue> = BTreeMap::new();
    for param in &profile.parameters {
        let at = format!("values.{}", param.name);
        let Some(value) = subject.values.get(&param.name) else {
            errors.push(ErrorItem::new(
                codes::MISSING_VALUE,
                at,
                "the subject supplies no value for this parameter",
            ));
            continue;
        };
        let Some(typed) = type_value(param.kind, value, &at, &mut errors) else {
            continue;
        };
        match &typed {
            TypedValue::Path(path) => check_path_value(path, &profile_roots, &at, &mut errors),
            TypedValue::PathList(paths) => {
                for (n, path) in paths.iter().enumerate() {
                    check_path_value(path, &profile_roots, &format!("{at}[{n}]"), &mut errors);
                }
            }
            TypedValue::Manifest(manifest) => {
                check_path_value(
                    &PathValue {
                        root: manifest.root.clone(),
                        path: manifest.path.clone(),
                    },
                    &profile_roots,
                    &at,
                    &mut errors,
                );
                if !is_sha256_hex(&manifest.sha256) {
                    errors.push(ErrorItem::new(
                        codes::INVALID_SHA256,
                        format!("{at}.sha256"),
                        "manifest sha256 is not 64 lowercase hex characters",
                    ));
                }
            }
            TypedValue::Str(_) | TypedValue::List(_) => {}
        }
        typed_values.insert(param.name.clone(), typed);
    }

    // Manifests: read, verify the digest, parse.
    let mut manifests: BTreeMap<String, Vec<ManifestEntry>> = BTreeMap::new();
    for param in &profile.parameters {
        if let Some(TypedValue::Manifest(value)) = typed_values.get(&param.name) {
            let at = format!("values.{}", param.name);
            if let Some(entries) = load_manifest(value, &subject.roots, &at, read, &mut errors)? {
                manifests.insert(param.name.clone(), entries);
            }
        }
    }

    // bound_by: root equals the manifest's entries_root, and the path is an entry.
    for param in &profile.parameters {
        let Some(target) = &param.bound_by else {
            continue;
        };
        let Some(entries) = manifests.get(target) else {
            continue;
        };
        let entries_root = params
            .get(target.as_str())
            .and_then(|p| p.entries_root.as_deref());
        let at = format!("values.{}", param.name);
        let listed: Vec<(String, &PathValue)> = match typed_values.get(&param.name) {
            Some(TypedValue::Path(path)) => vec![(at.clone(), path)],
            Some(TypedValue::PathList(paths)) => paths
                .iter()
                .enumerate()
                .map(|(n, path)| (format!("{at}[{n}]"), path))
                .collect(),
            _ => Vec::new(),
        };
        for (location, path) in listed {
            if Some(path.root.as_str()) != entries_root {
                errors.push(ErrorItem::new(
                    codes::BOUND_BY_ROOT_MISMATCH,
                    &location,
                    format!(
                        "root {:?} differs from entries_root {:?} of manifest {target:?}",
                        path.root,
                        entries_root.unwrap_or("")
                    ),
                ));
            }
            if !entries.iter().any(|entry| entry.path == path.path) {
                errors.push(ErrorItem::new(
                    codes::BOUND_BY_PATH_ABSENT,
                    &location,
                    format!(
                        "path {:?} is not an entry of manifest {target:?}",
                        path.path
                    ),
                ));
            }
        }
    }

    // Expanded git arguments obey the same rules as literal ones.
    for (n, requirement) in profile.requirements.iter().enumerate() {
        for (i, predicate) in requirement.predicates.iter().enumerate() {
            let args = match predicate {
                Predicate::GitOutputEquals(p) => &p.args,
                Predicate::GitOutputEmpty(p) => &p.args,
                _ => continue,
            };
            if args.iter().any(|a| matches!(a, StrVal::Param(_)))
                && let Some(tokens) = expand_git_args(args, &typed_values)
            {
                let at = format!("requirements[{n}].predicates[{i}].args");
                git_token_errors(&tokens, &at, &mut errors);
            }
        }
    }

    let typed = errors.is_empty().then(|| TypedSubject {
        roots: subject.roots.clone(),
        values: typed_values,
        manifests,
    });
    Ok(SubjectOutcome { errors, typed })
}

// ---------------------------------------------------------------------------
// Observation record
// ---------------------------------------------------------------------------

fn check_file_ref(reference: &FileRef, at: &str, errors: &mut Vec<ErrorItem>) {
    if !is_normalised_absolute(&reference.path) {
        errors.push(ErrorItem::new(
            codes::INVALID_ABSOLUTE_PATH,
            format!("{at}.path"),
            format!("{:?} is not an absolute normalised path", reference.path),
        ));
    }
    if !is_sha256_hex(&reference.sha256) {
        errors.push(ErrorItem::new(
            codes::INVALID_SHA256,
            format!("{at}.sha256"),
            "not 64 lowercase hex characters",
        ));
    }
}

/// Validate an observation record's own fields (contract 5.1). Digests of the files it
/// names are checked by `evaluate`, which has the files.
pub fn validate_observation(observation: &Observation) -> Vec<ErrorItem> {
    let mut errors = Vec::new();
    if observation.schema != OBSERVATION_SCHEMA {
        errors.push(ErrorItem::new(
            codes::SCHEMA_MISMATCH,
            "schema",
            format!("schema must be {OBSERVATION_SCHEMA:?}"),
        ));
    }
    check_file_ref(&observation.plan, "plan", &mut errors);
    check_file_ref(&observation.result, "result", &mut errors);
    check_file_ref(&observation.battery_script, "battery_script", &mut errors);
    if !matches!(observation.battery_exit, 0 | 1) {
        errors.push(ErrorItem::new(
            codes::BATTERY_EXIT_INVALID,
            "battery_exit",
            format!(
                "battery_exit {} is not a valid observation (only 0 or 1 is)",
                observation.battery_exit
            ),
        ));
    }
    if !is_sha256_hex(&observation.stdout_battery_sha256) {
        errors.push(ErrorItem::new(
            codes::INVALID_SHA256,
            "stdout_battery_sha256",
            "not 64 lowercase hex characters",
        ));
    }
    let started_ok = is_utc_timestamp(&observation.started_at);
    let finished_ok = is_utc_timestamp(&observation.finished_at);
    if !started_ok {
        errors.push(ErrorItem::new(
            codes::INVALID_TIMESTAMP,
            "started_at",
            "must be UTC YYYY-MM-DDTHH:MM:SSZ",
        ));
    }
    if !finished_ok {
        errors.push(ErrorItem::new(
            codes::INVALID_TIMESTAMP,
            "finished_at",
            "must be UTC YYYY-MM-DDTHH:MM:SSZ",
        ));
    }
    if started_ok && finished_ok && observation.finished_at < observation.started_at {
        errors.push(ErrorItem::new(
            codes::TIME_ORDER,
            "finished_at",
            "finished_at is before started_at",
        ));
    }
    if !is_normalised_absolute(&observation.invoker.script) {
        errors.push(ErrorItem::new(
            codes::INVALID_ABSOLUTE_PATH,
            "invoker.script",
            format!(
                "{:?} is not an absolute normalised path",
                observation.invoker.script
            ),
        ));
    }
    if !is_sha256_hex(&observation.invoker.sha256) {
        errors.push(ErrorItem::new(
            codes::INVALID_SHA256,
            "invoker.sha256",
            "not 64 lowercase hex characters",
        ));
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_patterns() {
        assert!(is_profile_id("manual-orchestration.integration-readiness"));
        assert!(!is_profile_id(".x"));
        assert!(is_requirement_id("M-source_1"));
        assert!(!is_requirement_id("M.source"));
        assert!(!is_requirement_id(""));
    }

    #[test]
    fn path_shapes() {
        assert!(is_normalised_absolute("/a/b"));
        for bad in ["/", "a/b", "/a/", "/a/./b", "/a/../b", "/a//b", ""] {
            assert!(!is_normalised_absolute(bad), "{bad}");
        }
        assert!(is_normalised_relative("a/b.txt"));
        for bad in ["", "/a", "a/", "a/../b", "./a", "a//b"] {
            assert!(!is_normalised_relative(bad), "{bad}");
        }
    }

    #[test]
    fn timestamps() {
        assert!(is_utc_timestamp("2026-10-21T12:00:03Z"));
        assert!(is_utc_timestamp("2028-02-29T00:00:00Z"));
        for bad in [
            "2026-02-29T00:00:00Z",
            "2026-13-01T00:00:00Z",
            "2026-10-21T24:00:00Z",
            "2026-10-21 12:00:03Z",
            "2026-10-21T12:00:03+00:00",
            "2026-10-21T12:00:03",
        ] {
            assert!(!is_utc_timestamp(bad), "{bad}");
        }
    }

    #[test]
    fn git_tokens() {
        let mut errors = Vec::new();
        git_token_errors(
            &[
                "status".into(),
                "--porcelain".into(),
                "--".into(),
                "a".into(),
            ],
            "x",
            &mut errors,
        );
        assert!(errors.is_empty());
        git_token_errors(&["push".into()], "x", &mut errors);
        git_token_errors(&["status".into(), "--output=x".into()], "x", &mut errors);
        git_token_errors(
            &["status".into(), "--".into(), "-x".into()],
            "x",
            &mut errors,
        );
        git_token_errors(&["status".into(), "-c".into()], "x", &mut errors);
        let codes: Vec<_> = errors.iter().map(|e| e.code.as_str()).collect();
        assert_eq!(
            codes,
            [
                "git_subcommand_forbidden",
                "git_argument_forbidden",
                "git_argument_forbidden",
                "git_argument_forbidden"
            ]
        );
    }
}
