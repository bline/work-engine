//! Closed serde types for the five document kinds (contract 3, 5, 6).
//!
//! Every document type carries `deny_unknown_fields`; the one exception is a
//! manifest entry (contract 3.3), which other owners produce and which `plan`
//! reads through [`ManifestEntry`], a deliberately open type.
//!
//! Serialisation shape (contract 2.3): structs with declared field order,
//! `Vec`, and `BTreeMap` only. There is no `HashMap` or `HashSet` here.
//!
//! [`Assessment`] and the types only it uses derive `Deserialize` and not
//! `Serialize`, so this crate has no way to write an assessment (AC5 b).

use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use std::collections::BTreeMap;

pub const PROFILE_SCHEMA: &str = "decision-readiness-profile-v1";
pub const SUBJECT_SCHEMA: &str = "decision-readiness-subject-v1";
pub const OBSERVATION_SCHEMA: &str = "decision-readiness-observation-v1";
pub const EVALUATION_SCHEMA: &str = "decision-readiness-evaluation-v1";
pub const ASSESSMENT_SCHEMA: &str = "decision-readiness-assessment-v1";

// ---------------------------------------------------------------------------
// Shared small types
// ---------------------------------------------------------------------------

/// `{id, owner_role}` of the decision a profile serves (contract 3.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub id: String,
    pub owner_role: String,
}

/// `{id, version, sha256}` naming a profile by its file bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileRef {
    pub id: String,
    pub version: u64,
    pub sha256: String,
}

/// `{path, sha256}`: an absolute path and the sha256 of its bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileRef {
    pub path: String,
    pub sha256: String,
}

/// Requirement or predicate status (contract 3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Met,
    Unmet,
    Indeterminate,
}

// ---------------------------------------------------------------------------
// Profile (contract 3.1)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub schema: String,
    pub profile_id: String,
    pub version: u64,
    pub decision: Decision,
    pub profile_owner: String,
    pub assessor_roles: Vec<String>,
    pub authority_refs: Vec<AuthorityRef>,
    pub roots: Vec<RootDecl>,
    pub parameters: Vec<ParamDecl>,
    pub requirements: Vec<Requirement>,
}

/// `lines` is a JSON string, for example `"369-373"` (contract 3.7 item 2).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorityRef {
    pub path: String,
    pub lines: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootDecl {
    pub name: String,
    #[serde(default)]
    pub git: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamKind {
    String,
    List,
    Path,
    PathList,
    Manifest,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParamDecl {
    pub name: String,
    pub kind: ParamKind,
    /// Required for `manifest`: the root its entries resolve against.
    #[serde(default)]
    pub entries_root: Option<String>,
    /// Allowed on `path` and `path_list`: the manifest parameter that must list the path.
    #[serde(default)]
    pub bound_by: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementClass {
    Mechanical,
    Attributed,
}

/// A requirement carries `id`, `class`, `statement`, and `predicates` (mechanical)
/// or `evidence_from` (attributed). It has no `rationale` field (contract 3.7 item 3).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    pub id: String,
    pub class: RequirementClass,
    pub statement: String,
    #[serde(default)]
    pub predicates: Vec<Predicate>,
    #[serde(default)]
    pub evidence_from: Vec<String>,
}

/// `{"param": "<name>"}`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParamRef {
    pub param: String,
}

/// `{root, path}` with a literal root name and a relative path.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RootPath {
    pub root: String,
    pub path: String,
}

/// `{root, path}` or `{param}` of kind `path`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum PathRef {
    Literal(RootPath),
    Param(ParamRef),
}

/// A string literal or `{param}` of kind `string` (or `list`, inside `git` args).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum StrVal {
    Literal(String),
    Param(ParamRef),
}

/// `json_equals.expected`: `{param}` of kind `string`, or a JSON literal.
///
/// A JSON object with exactly the key `param` and a string value is always a
/// parameter reference, so that object cannot be written as a literal.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Expected {
    Param(ParamRef),
    Literal(Value),
}

/// `grep_count.files`: an array of PathRefs, or `{param}` naming a `path_list`
/// parameter as the whole value (contract 3.7 item 1).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum FilesSpec {
    Param(ParamRef),
    List(Vec<PathRef>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompareOp {
    Eq,
    Le,
    Ge,
}

/// The eight closed predicate kinds (contract 3.5).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Predicate {
    FileSha256(FileSha256),
    ManifestMatch(ManifestMatch),
    JsonEquals(JsonEquals),
    JsonCompare(JsonCompare),
    JsonPresent(JsonPresent),
    GrepCount(GrepCount),
    GitOutputEquals(GitOutputEquals),
    GitOutputEmpty(GitOutputEmpty),
}

impl Predicate {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Predicate::FileSha256(_) => "file_sha256",
            Predicate::ManifestMatch(_) => "manifest_match",
            Predicate::JsonEquals(_) => "json_equals",
            Predicate::JsonCompare(_) => "json_compare",
            Predicate::JsonPresent(_) => "json_present",
            Predicate::GrepCount(_) => "grep_count",
            Predicate::GitOutputEquals(_) => "git_output_equals",
            Predicate::GitOutputEmpty(_) => "git_output_empty",
        }
    }

    pub fn is_git(&self) -> bool {
        matches!(
            self,
            Predicate::GitOutputEquals(_) | Predicate::GitOutputEmpty(_)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileSha256 {
    pub file: PathRef,
    pub expected: StrVal,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestMatch {
    pub manifest: ParamRef,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonEquals {
    pub file: PathRef,
    pub key: String,
    #[serde(default)]
    pub each: Option<ParamRef>,
    pub expected: Expected,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonCompare {
    pub file: PathRef,
    pub key: String,
    #[serde(default)]
    pub each: Option<ParamRef>,
    pub op: CompareOp,
    pub value: Number,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JsonPresent {
    pub file: PathRef,
    pub key: String,
    #[serde(default)]
    pub each: Option<ParamRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrepCount {
    pub pattern: String,
    pub files: FilesSpec,
    #[serde(default)]
    pub min: Option<u64>,
    #[serde(default)]
    pub max: Option<u64>,
    #[serde(default)]
    pub per_path: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitOutputEquals {
    pub args: Vec<StrVal>,
    pub expected: StrVal,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GitOutputEmpty {
    pub args: Vec<StrVal>,
}

// ---------------------------------------------------------------------------
// Subject (contract 3.2, 3.3)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub schema: String,
    pub profile: ProfileRef,
    pub subject_id: String,
    pub author: String,
    pub roots: BTreeMap<String, String>,
    /// Typed by the profile's parameter kinds; see `validate::validate_subject`.
    pub values: BTreeMap<String, Value>,
}

/// A `path` value: `{root, path}`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathValue {
    pub root: String,
    pub path: String,
}

/// A `manifest` value: `{root, path, sha256}` for the manifest file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestValue {
    pub root: String,
    pub path: String,
    pub sha256: String,
}

/// A manifest entry. Open on purpose: other keys are ignored and not compared
/// (contract 3.3), so there is no `deny_unknown_fields` here.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ManifestEntry {
    pub path: String,
    pub sha256: String,
}

// ---------------------------------------------------------------------------
// Observation record (contract 5.1)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub schema: String,
    pub plan: FileRef,
    pub result: FileRef,
    pub battery_script: FileRef,
    pub battery_exit: i64,
    pub stdout_battery_sha256: String,
    pub started_at: String,
    pub finished_at: String,
    pub invoker: Invoker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Invoker {
    pub script: String,
    pub sha256: String,
}

// ---------------------------------------------------------------------------
// Evaluation (contract 5.3). Produced by `evaluate` in a later slice. It has no
// verdict field, and the closed schema rejects one.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evaluation {
    pub schema: String,
    pub profile: ProfileRef,
    pub subject: SubjectRef,
    pub run_plan: FileRef,
    pub battery: BatteryRecord,
    pub observation: ObservationRecord,
    pub tool: ToolRecord,
    pub evaluated_at: String,
    pub derived_check_ids: Vec<String>,
    pub binding: Binding,
    pub requirements: Vec<EvaluatedRequirement>,
    pub counts: Counts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubjectRef {
    pub subject_id: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatteryRecord {
    pub path: String,
    pub file_sha256: String,
    pub battery_sha256: String,
    pub failed: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationRecord {
    pub path: String,
    pub sha256: String,
    pub started_at: String,
    pub finished_at: String,
    pub battery_exit: i64,
    pub battery_script_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolRecord {
    pub name: String,
    pub version: String,
    pub binary_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitRootBinding {
    Verified,
    Mismatch,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EchoBinding {
    EchoVerified,
    Partial,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub git_root: GitRootBinding,
    pub checks: BTreeMap<String, EchoBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluatedRequirement {
    pub id: String,
    pub class: EvaluatedClass,
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub predicates: Vec<PredicateResult>,
    #[serde(default)]
    pub evidence: Vec<EvidenceItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluatedClass {
    Mechanical,
    Attributed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredicateResult {
    pub index: u64,
    pub kind: String,
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceItem {
    pub check_id: String,
    pub kind: String,
    pub digest_field: String,
    pub digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Counts {
    pub met: u64,
    pub unmet: u64,
    pub indeterminate: u64,
}

// ---------------------------------------------------------------------------
// Assessment (contract 6). Deserialize only: this crate cannot write one.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    Live,
    Fixture,
    Demonstration,
}

impl Purpose {
    pub fn as_str(self) -> &'static str {
        match self {
            Purpose::Live => "live",
            Purpose::Fixture => "fixture",
            Purpose::Demonstration => "demonstration",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerdictValue {
    Ready,
    Blocked,
    Uncertain,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assessment {
    pub schema: String,
    pub purpose: Purpose,
    pub assessment_id: String,
    pub issued_at: String,
    pub assessor: Assessor,
    pub decision: Decision,
    pub profile: ProfileRef,
    pub evaluation: FileRef,
    pub verdict: VerdictValue,
    pub requirements: Vec<AssessedRequirement>,
    pub missing: Vec<String>,
    pub indeterminate: Vec<String>,
    pub profile_gaps: Vec<ProfileGap>,
    pub evidence: Vec<FileRef>,
    pub freshness_basis: FreshnessBasis,
    pub limits: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assessor {
    pub name: String,
    pub role: String,
    pub session: String,
    pub model_requested: String,
    pub model_observed: String,
    pub authority_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentBasis {
    Evaluation,
    Assessor,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssessedRequirement {
    pub id: String,
    pub evaluation_status: Status,
    pub assessed_status: Status,
    pub basis: AssessmentBasis,
    pub reason: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileGap {
    pub statement: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FreshnessBasis {
    pub observation: FreshnessObservation,
    /// Typed claim reliance (claim id plus exact revision); empty for C1 (contract D5).
    pub claim_reliance: Vec<ClaimReliance>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FreshnessObservation {
    pub battery_file_sha256: String,
    pub battery_sha256: String,
    pub observation_sha256: String,
    pub started_at: String,
    pub finished_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimReliance {
    pub claim_id: String,
    pub revision: String,
}
