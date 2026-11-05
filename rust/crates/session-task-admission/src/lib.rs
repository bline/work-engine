//! Review-binding validation for proposed task requests.
//!
//! This library does not grant authority or dispatch work. A valid review
//! document is only one prerequisite for a separately verified admission.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub struct Ref {
    pub id: String,
    pub revision: String,
    pub sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    r#ref: Ref,
    content_json: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Workspace {
    profile_id: String,
    checkpoint: String,
    configuration: Ref,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Model {
    id: String,
    effort: String,
    tier: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Effects {
    write_paths: Vec<String>,
    network: bool,
    max_concurrent_tasks: u64,
    max_child_tasks: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Clock {
    first_funded_rule: String,
    admit_until: String,
    close_by: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Budget {
    objective_id: String,
    units: String,
    maximum: String,
    productive_maximum: String,
    closure_reserve: String,
    coverage: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Authority {
    mode: Ref,
    #[serde(deserialize_with = "required_option")]
    mandate: Option<Ref>,
    allowed_operations: Vec<String>,
    reserved_decisions: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    schema: String,
    task_id: String,
    lane_id: String,
    revision: String,
    proposal: Ref,
    objective: String,
    input: Ref,
    operation: String,
    provider: String,
    execution_profile: String,
    workspace: Workspace,
    model: Model,
    effects: Effects,
    prerequisites: Vec<Ref>,
    clock: Clock,
    budget: Budget,
    submitter: String,
    authority: Authority,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ModeView {
    schema: String,
    owner_id: String,
    revision: String,
    mode: String,
    effective_at: String,
    #[serde(deserialize_with = "required_option")]
    pending_transition: Option<serde_json::Value>,
    mandates: Vec<Ref>,
    in_flight: Vec<Ref>,
    observed_at: String,
    source_revision: String,
}
#[derive(Deserialize)]
struct Data {
    intent: Intent,
    intent_artifacts: Vec<Artifact>,
}
#[derive(Deserialize)]
struct Row {
    data: Data,
}

fn required_option<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(d)
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReviewError {
    InvalidJson,
    Unsupported,
    InvalidReference,
    MissingArtifact,
    ConflictingArtifact,
}

fn nonempty(s: &str) -> bool {
    !s.is_empty() && !s.contains('\0')
}
fn valid_ref(r: &Ref) -> bool {
    nonempty(&r.id)
        && nonempty(&r.revision)
        && r.sha256.len() == 64
        && r.sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn relative_write_path(s: &str) -> bool {
    nonempty(s)
        && !s.contains('\\')
        && std::path::Path::new(s)
            .components()
            .all(|c| matches!(c, std::path::Component::Normal(_)))
}
fn decimal(s: &str) -> bool {
    let mut parts = s.split('.');
    let whole = parts.next().unwrap_or("");
    !whole.is_empty()
        && whole.bytes().all(|b| b.is_ascii_digit())
        && parts
            .next()
            .is_none_or(|f| !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()))
        && parts.next().is_none()
}
fn budget_within(maximum: &str, productive: &str, reserve: &str) -> bool {
    if ![maximum, productive, reserve].into_iter().all(decimal) {
        return false;
    }
    let scale = [maximum, productive, reserve]
        .into_iter()
        .map(|s| s.split_once('.').map_or(0, |(_, f)| f.len()))
        .max()
        .unwrap_or(0);
    fn units(s: &str, scale: usize) -> Vec<u8> {
        let (whole, fraction) = s.split_once('.').unwrap_or((s, ""));
        let mut digits = whole
            .bytes()
            .chain(fraction.bytes())
            .map(|b| b - b'0')
            .collect::<Vec<_>>();
        digits.resize(digits.len() + scale - fraction.len(), 0);
        digits.reverse();
        digits
    }
    let max = units(maximum, scale);
    let p = units(productive, scale);
    let r = units(reserve, scale);
    let mut sum = Vec::new();
    let mut carry = 0;
    for i in 0..p.len().max(r.len()) {
        let n = p.get(i).copied().unwrap_or(0) + r.get(i).copied().unwrap_or(0) + carry;
        sum.push(n % 10);
        carry = n / 10;
    }
    if carry > 0 {
        sum.push(carry);
    }
    fn trim(mut d: Vec<u8>) -> Vec<u8> {
        while d.len() > 1 && d.last() == Some(&0) {
            d.pop();
        }
        d
    }
    let max = trim(max);
    let sum = trim(sum);
    sum.len() < max.len()
        || (sum.len() == max.len() && sum.iter().rev().cmp(max.iter().rev()).is_le())
}
fn utc_time(s: &str) -> Option<time::OffsetDateTime> {
    let t = time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok()?;
    (t.offset() == time::UtcOffset::UTC).then_some(t)
}
fn hash(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}

/// Validate only the review document and exact artifact bytes. Does not check
/// signatures, current state, profile realization, clocks, or budget authority.
pub fn validate_review(raw_request: &str) -> Result<Ref, ReviewError> {
    let row: Row = serde_json::from_str(raw_request).map_err(|_| ReviewError::InvalidJson)?;
    let i = row.data.intent;
    if !matches!(
        (
            i.schema.as_str(),
            i.provider.as_str(),
            i.execution_profile.as_str()
        ),
        (
            "dot.task-intent/v1-proposal",
            "codex",
            "disposable-codex-write-v1"
        ) | (
            "dot.task-intent/pi-coding-v1",
            "pi",
            "disposable-pi-coding-fixture-v1"
        ) | (
            "dot.task-intent/pi-coding-subscription-v1",
            "pi",
            "disposable-pi-coding-subscription-v1"
        )
    ) || i.operation != "start"
        || (i.schema == "dot.task-intent/pi-coding-v1"
            && (i.effects.write_paths != ["src/lib.rs"]
                || i.workspace.profile_id != pi_pilot::PROFILE
                || i.model.id != "fake-pi-model"
                || i.model.effort != "off"
                || i.model.tier != "fixture"))
        || (i.schema == "dot.task-intent/pi-coding-subscription-v1"
            && (i.effects.write_paths != ["src/lib.rs"]
                || i.workspace.profile_id != pi_pilot::SUBSCRIPTION_PROFILE
                || i.model.id != "gpt-6-luna"
                || i.model.effort != "off"
                || i.model.tier != "subscription"))
        || i.effects.network != (i.schema == "dot.task-intent/pi-coding-subscription-v1")
        || i.effects.max_concurrent_tasks != 1
        || i.effects.max_child_tasks != 0
        || i.clock.first_funded_rule != "first_funded_setup_or_readiness"
        || i.budget.coverage != "all_objective_actors"
        || i.authority.mandate.is_some()
        || i.authority.allowed_operations != ["start"]
        || i.prerequisites.is_empty()
        || !budget_within(
            &i.budget.maximum,
            &i.budget.productive_maximum,
            &i.budget.closure_reserve,
        )
        || !i
            .budget
            .maximum
            .bytes()
            .any(|b| b.is_ascii_digit() && b != b'0')
        || !matches!((utc_time(&i.clock.admit_until),utc_time(&i.clock.close_by)),(Some(a),Some(b)) if b>a)
    {
        return Err(ReviewError::Unsupported);
    }
    if [
        &i.task_id,
        &i.lane_id,
        &i.revision,
        &i.objective,
        &i.workspace.profile_id,
        &i.workspace.checkpoint,
        &i.model.id,
        &i.model.effort,
        &i.model.tier,
        &i.clock.admit_until,
        &i.clock.close_by,
        &i.budget.objective_id,
        &i.budget.units,
        &i.budget.maximum,
        &i.budget.productive_maximum,
        &i.budget.closure_reserve,
        &i.submitter,
    ]
    .into_iter()
    .any(|s| !nonempty(s))
        || i.effects.write_paths.is_empty()
        || i.effects
            .write_paths
            .iter()
            .any(|s| !relative_write_path(s))
        || i.authority.reserved_decisions.iter().any(|s| !nonempty(s))
    {
        return Err(ReviewError::Unsupported);
    }
    let mut artifacts = HashMap::new();
    let mut identities = HashMap::new();
    for a in row.data.intent_artifacts {
        if !valid_ref(&a.r#ref)
            || hash(&a.content_json) != a.r#ref.sha256
            || serde_json::from_str::<serde_json::Value>(&a.content_json).is_err()
        {
            return Err(ReviewError::InvalidReference);
        }
        let identity = (a.r#ref.id.clone(), a.r#ref.revision.clone());
        if identities
            .insert(identity, a.r#ref.sha256.clone())
            .is_some()
            || artifacts.insert(a.r#ref, a.content_json).is_some()
        {
            return Err(ReviewError::ConflictingArtifact);
        }
    }
    for r in [
        &i.proposal,
        &i.input,
        &i.workspace.configuration,
        &i.authority.mode,
    ]
    .into_iter()
    .chain(i.prerequisites.iter())
    {
        if !valid_ref(r) {
            return Err(ReviewError::InvalidReference);
        }
        if !artifacts.contains_key(r) {
            return Err(ReviewError::MissingArtifact);
        }
    }
    let mode: ModeView = serde_json::from_str(
        artifacts
            .get(&i.authority.mode)
            .ok_or(ReviewError::MissingArtifact)?,
    )
    .map_err(|_| ReviewError::Unsupported)?;
    if mode.schema != "dot.mode-view/v1-proposal"
        || mode.mode != "present"
        || mode.revision != i.authority.mode.revision
        || !nonempty(&mode.owner_id)
        || !nonempty(&mode.source_revision)
        || mode.pending_transition.is_some()
        || mode
            .mandates
            .iter()
            .chain(&mode.in_flight)
            .any(|r| !valid_ref(r))
        || utc_time(&mode.effective_at).is_none()
        || utc_time(&mode.observed_at).is_none()
    {
        return Err(ReviewError::Unsupported);
    }
    Ok(Ref {
        id: i.task_id,
        revision: i.revision,
        sha256: hash(raw_request),
    })
}

pub mod proof;

mod store;

mod current;

pub mod projection;

pub mod result;

mod key_policy;

mod board;

pub mod request;

mod accounting;

mod source;

mod inspect;

pub mod service;

mod admission;

/// Typed canonical-source custody; conveys no human or execution authority.
pub mod canonical_source;
/// Authenticated admission-owner recollection, disabled pending qualification.
pub mod native_authority;
/// Signed scoped authority observations; native custody remains bridge-owned.
pub mod scoped_pilot;

/// Fixture-only signed Pi composition; never qualifies the ordinary native gate.
pub mod pi_pilot;

/// Synthetic task projection and canonical signed-answer consumption only.
pub mod owner_boundary;
