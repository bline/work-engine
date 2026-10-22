//! Deterministic plan derivation (contract 4).
//!
//! `derive_plan` is a pure function of the profile bytes, the subject bytes and the
//! bytes of the manifests the subjects name. It reads nothing else and holds no clock.
//! The output holds exactly `root`, `extra_roots` and `checks`, so the evidence
//! battery runs it unchanged.

use crate::diag::{ErrorItem, codes};
use crate::digest::sha256_hex;
use crate::model::*;
use crate::validate::{
    DocError, EACH_PLACEHOLDER, ReadError, ReadFile, TypedSubject, TypedValue, absolute_path,
    expand_git_args, parse_document, validate_profile, validate_subject,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

/// An evidence-battery plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Plan {
    pub root: String,
    pub extra_roots: Vec<String>,
    pub checks: Vec<PlanCheck>,
}

/// One battery check: an id plus exactly one of `sha256`, `json`, `grep`, `git`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlanCheck {
    pub id: String,
    #[serde(flatten)]
    pub spec: CheckSpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckSpec {
    Sha256(Vec<String>),
    Json(JsonSpec),
    Grep(GrepSpec),
    Git(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JsonSpec {
    pub path: String,
    pub select: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GrepSpec {
    pub pattern: String,
    pub paths: Vec<String>,
}

/// Why `plan` produced no plan.
#[derive(Debug, Error)]
pub enum PlanError {
    /// A document is not JSON, or a named file cannot be read: exit 2.
    #[error("{0}")]
    Input(String),
    /// A document violates a rule: exit 2, with the errors listed.
    #[error("plan refused: {} violated rule(s)", .0.len())]
    Refused(Vec<ErrorItem>),
}

impl From<ReadError> for PlanError {
    fn from(error: ReadError) -> Self {
        PlanError::Input(error.to_string())
    }
}

/// One profile and subject pair, as file bytes.
pub struct PairBytes<'a> {
    pub profile: &'a [u8],
    pub subject: &'a [u8],
}

struct Pair {
    profile: Profile,
    subject: TypedSubject,
}

fn internal(at: &str, message: &str) -> ErrorItem {
    ErrorItem::new(codes::INTERNAL, at, message)
}

fn parse_error(label: &str, error: DocError) -> PlanError {
    match error {
        DocError::Syntax(message) => PlanError::Input(format!("{label}: {message}")),
        DocError::Schema(message) => {
            PlanError::Refused(vec![ErrorItem::new(codes::SCHEMA, label, message)])
        }
    }
}

/// Derive a battery plan from one or more profile and subject pairs (contract 4).
pub fn derive_plan(pairs: &[PairBytes<'_>], read: ReadFile<'_>) -> Result<Plan, PlanError> {
    if pairs.is_empty() {
        return Err(PlanError::Input(
            "plan needs at least one profile and subject".into(),
        ));
    }
    let mut loaded = Vec::with_capacity(pairs.len());
    let mut errors: Vec<ErrorItem> = Vec::new();
    for (n, bytes) in pairs.iter().enumerate() {
        let profile_label = format!("pair[{n}].profile");
        let subject_label = format!("pair[{n}].subject");
        let profile: Profile =
            parse_document(bytes.profile).map_err(|e| parse_error(&profile_label, e))?;
        let subject: Subject =
            parse_document(bytes.subject).map_err(|e| parse_error(&subject_label, e))?;
        let profile_errors = validate_profile(&profile);
        if !profile_errors.is_empty() {
            // A subject cannot be typed against an invalid profile.
            errors.extend(
                profile_errors
                    .into_iter()
                    .map(|e| e.prefixed(&profile_label)),
            );
            continue;
        }
        let outcome = validate_subject(&profile, &sha256_hex(bytes.profile), &subject, read)?;
        errors.extend(
            outcome
                .errors
                .into_iter()
                .map(|e| e.prefixed(&subject_label)),
        );
        if let Some(typed) = outcome.typed {
            loaded.push(Pair {
                profile,
                subject: typed,
            });
        }
    }
    if !errors.is_empty() {
        return Err(PlanError::Refused(errors));
    }
    cross_pair_errors(&loaded)?;
    build(&loaded).map_err(|e| PlanError::Refused(vec![e]))
}

/// Multi-profile rules (contract 4): no repeated (id, version), shared root names map
/// to one path, and at most one distinct git-root path.
fn cross_pair_errors(pairs: &[Pair]) -> Result<(), PlanError> {
    let mut errors = Vec::new();
    let mut seen_profiles = BTreeSet::new();
    let mut name_to_path: BTreeMap<&str, &str> = BTreeMap::new();
    let mut git_paths: Vec<&str> = Vec::new();
    for (n, pair) in pairs.iter().enumerate() {
        if !seen_profiles.insert((pair.profile.profile_id.as_str(), pair.profile.version)) {
            errors.push(ErrorItem::new(
                codes::DUPLICATE_PROFILE,
                format!("pair[{n}]"),
                format!(
                    "profile {} version {} appears twice",
                    pair.profile.profile_id, pair.profile.version
                ),
            ));
        }
        for root in &pair.profile.roots {
            let Some(path) = pair.subject.roots.get(&root.name) else {
                continue;
            };
            match name_to_path.get(root.name.as_str()) {
                Some(previous) if previous != &path.as_str() => errors.push(ErrorItem::new(
                    codes::ROOT_PATH_CONFLICT,
                    format!("pair[{n}].subject.roots.{}", root.name),
                    format!("root {:?} maps to {previous:?} and {path:?}", root.name),
                )),
                Some(_) => {}
                None => {
                    name_to_path.insert(root.name.as_str(), path.as_str());
                }
            }
            if root.git && !git_paths.contains(&path.as_str()) {
                git_paths.push(path.as_str());
            }
        }
    }
    if git_paths.len() > 1 {
        errors.push(ErrorItem::new(
            codes::MULTIPLE_GIT_ROOT_PATHS,
            "roots",
            format!("a plan may name one git root path; found {git_paths:?}"),
        ));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(PlanError::Refused(errors))
    }
}

fn build(pairs: &[Pair]) -> Result<Plan, ErrorItem> {
    // Roots (contract 3.4 rule 2).
    let mut git_root_path: Option<&str> = None;
    for pair in pairs {
        for root in &pair.profile.roots {
            if root.git && git_root_path.is_none() {
                git_root_path = pair.subject.roots.get(&root.name).map(String::as_str);
            }
        }
    }
    let first_path = pairs
        .first()
        .and_then(|pair| pair.profile.roots.first().map(|root| (pair, root)))
        .and_then(|(pair, root)| pair.subject.roots.get(&root.name))
        .ok_or_else(|| internal("roots", "the first pair declares no root"))?;
    let root = git_root_path.unwrap_or(first_path.as_str()).to_string();
    let mut extra_roots: Vec<String> = Vec::new();
    for pair in pairs {
        for decl in &pair.profile.roots {
            if let Some(path) = pair.subject.roots.get(&decl.name)
                && *path != root
                && !extra_roots.contains(path)
            {
                extra_roots.push(path.clone());
            }
        }
    }

    let mut checks = Vec::new();
    for pair in pairs {
        derive_pair(pair, &mut checks)?;
    }
    Ok(Plan {
        root,
        extra_roots,
        checks,
    })
}

struct PairView<'a> {
    pair: &'a Pair,
}

impl PairView<'_> {
    fn root_path(&self, name: &str) -> Result<&str, ErrorItem> {
        self.pair
            .subject
            .roots
            .get(name)
            .map(String::as_str)
            .ok_or_else(|| internal(name, "root has no path"))
    }

    fn abs(&self, root: &str, relative: &str) -> Result<String, ErrorItem> {
        Ok(absolute_path(self.root_path(root)?, relative))
    }

    fn value(&self, name: &str) -> Result<&TypedValue, ErrorItem> {
        self.pair
            .subject
            .values
            .get(name)
            .ok_or_else(|| internal(name, "parameter has no value"))
    }

    fn path_ref(&self, reference: &PathRef) -> Result<String, ErrorItem> {
        match reference {
            PathRef::Literal(literal) => self.abs(&literal.root, &literal.path),
            PathRef::Param(param) => match self.value(&param.param)? {
                TypedValue::Path(path) => self.abs(&path.root, &path.path),
                _ => Err(internal(&param.param, "expected a path value")),
            },
        }
    }

    fn list(&self, name: &str) -> Result<&[String], ErrorItem> {
        match self.value(name)? {
            TypedValue::List(items) => Ok(items),
            _ => Err(internal(name, "expected a list value")),
        }
    }

    fn manifest(&self, name: &str) -> Result<(&ManifestValue, &[ManifestEntry]), ErrorItem> {
        let value = match self.value(name)? {
            TypedValue::Manifest(value) => value,
            _ => return Err(internal(name, "expected a manifest value")),
        };
        let entries = self
            .pair
            .subject
            .manifests
            .get(name)
            .ok_or_else(|| internal(name, "manifest was not loaded"))?;
        Ok((value, entries))
    }

    /// The selected keys: the key once, or one per element of the `each` list.
    fn select(&self, key: &str, each: &Option<ParamRef>) -> Result<Vec<String>, ErrorItem> {
        match each {
            None => Ok(vec![key.to_string()]),
            Some(param) => Ok(self
                .list(&param.param)?
                .iter()
                .map(|element| key.replace(EACH_PLACEHOLDER, element))
                .collect()),
        }
    }
}

fn derive_pair(pair: &Pair, checks: &mut Vec<PlanCheck>) -> Result<(), ErrorItem> {
    let view = PairView { pair };
    let profile = &pair.profile;
    let prefix = format!("{}.v{}", profile.profile_id, profile.version);
    let mut push = |id: String, spec: CheckSpec| checks.push(PlanCheck { id, spec });

    if profile.roots.iter().any(|root| root.git) {
        push(
            format!("{prefix}._git_root"),
            CheckSpec::Git(vec!["rev-parse".to_string(), "--show-toplevel".to_string()]),
        );
    }

    for requirement in &profile.requirements {
        if requirement.class != RequirementClass::Mechanical {
            continue;
        }
        for (index, predicate) in requirement.predicates.iter().enumerate() {
            let id = format!("{prefix}.{}.{}", requirement.id, index + 1);
            match predicate {
                Predicate::FileSha256(p) => {
                    push(id, CheckSpec::Sha256(vec![view.path_ref(&p.file)?]));
                }
                Predicate::ManifestMatch(p) => {
                    let (value, entries) = view.manifest(&p.manifest.param)?;
                    let entries_root = profile
                        .parameters
                        .iter()
                        .find(|param| param.name == p.manifest.param)
                        .and_then(|param| param.entries_root.as_deref())
                        .ok_or_else(|| {
                            internal(&p.manifest.param, "manifest has no entries_root")
                        })?;
                    let paths = entries
                        .iter()
                        .map(|entry| view.abs(entries_root, &entry.path))
                        .collect::<Result<Vec<_>, _>>()?;
                    let manifest_file = view.abs(&value.root, &value.path)?;
                    push(id.clone(), CheckSpec::Sha256(paths));
                    push(
                        format!("{id}.manifest"),
                        CheckSpec::Sha256(vec![manifest_file]),
                    );
                }
                Predicate::JsonEquals(p) => {
                    push_json(&mut push, &view, id, &p.file, &p.key, &p.each)?;
                }
                Predicate::JsonCompare(p) => {
                    push_json(&mut push, &view, id, &p.file, &p.key, &p.each)?;
                }
                Predicate::JsonPresent(p) => {
                    push_json(&mut push, &view, id, &p.file, &p.key, &p.each)?;
                }
                Predicate::GrepCount(p) => {
                    let paths = match &p.files {
                        FilesSpec::Param(param) => match view.value(&param.param)? {
                            TypedValue::PathList(items) => items
                                .iter()
                                .map(|item| view.abs(&item.root, &item.path))
                                .collect::<Result<Vec<_>, _>>()?,
                            _ => return Err(internal(&param.param, "expected a path_list value")),
                        },
                        FilesSpec::List(list) => list
                            .iter()
                            .map(|reference| view.path_ref(reference))
                            .collect::<Result<Vec<_>, _>>()?,
                    };
                    if p.per_path {
                        for (j, path) in paths.iter().enumerate() {
                            push(
                                format!("{id}.{}", j + 1),
                                CheckSpec::Grep(GrepSpec {
                                    pattern: p.pattern.clone(),
                                    paths: vec![path.clone()],
                                }),
                            );
                        }
                    } else {
                        push(
                            id.clone(),
                            CheckSpec::Grep(GrepSpec {
                                pattern: p.pattern.clone(),
                                paths: paths.clone(),
                            }),
                        );
                    }
                    push(format!("{id}.exists"), CheckSpec::Sha256(paths));
                }
                Predicate::GitOutputEquals(p) => {
                    push(id, CheckSpec::Git(git_args(&view, &p.args)?));
                }
                Predicate::GitOutputEmpty(p) => {
                    push(id, CheckSpec::Git(git_args(&view, &p.args)?));
                }
            }
        }
    }
    Ok(())
}

fn git_args(view: &PairView<'_>, args: &[StrVal]) -> Result<Vec<String>, ErrorItem> {
    expand_git_args(args, &view.pair.subject.values)
        .ok_or_else(|| internal("args", "a git argument has no value"))
}

fn push_json(
    push: &mut impl FnMut(String, CheckSpec),
    view: &PairView<'_>,
    id: String,
    file: &PathRef,
    key: &str,
    each: &Option<ParamRef>,
) -> Result<(), ErrorItem> {
    let path = view.path_ref(file)?;
    push(
        id.clone(),
        CheckSpec::Json(JsonSpec {
            path: path.clone(),
            select: view.select(key, each)?,
        }),
    );
    push(format!("{id}.exists"), CheckSpec::Sha256(vec![path]));
    Ok(())
}

/// Serialise a plan: `serde_json::to_string_pretty` plus one `\n` (contract 2.3).
pub fn plan_to_string(plan: &Plan) -> Result<String, serde_json::Error> {
    let mut text = serde_json::to_string_pretty(plan)?;
    text.push('\n');
    Ok(text)
}
