//! Composition seam for a future qualified protected context. There is no
//! production constructor for `QualifiedContext`: no caller-supplied JSON,
//! OAuth identity, local credential or requested sandbox can create one.
use crate::{
    Ref, Row,
    current::{self, Current},
    inspect::{self, Held, Sources},
    store::{self, Admission},
};
use rusqlite::Connection;
use time::{Duration, OffsetDateTime};

/// The eventual owner must construct this only from independently established
/// current mode, exclusive objective/controller coverage and actual native
/// profile evidence. No constructor is provided in production builds.
#[allow(dead_code)]
pub(crate) struct QualifiedContext {
    task: Ref,
    proposal: Ref,
    mode: Ref,
    lane: String,
    actor: String,
    mode_effective: OffsetDateTime,
    expires: OffsetDateTime,
    profile_id: String,
    profile_paths: Vec<String>,
    model: String,
    effort: String,
    tier: String,
    exclusive_evidence: String,
}
pub(crate) fn gate(
    _config: &Sources,
    evidence: &inspect::Evidence,
    q: &QualifiedContext,
    now: OffsetDateTime,
) -> Option<store::CurrentGate> {
    if q.expires <= now || q.exclusive_evidence.is_empty() {
        return None;
    }
    let row: Row = serde_json::from_str(&evidence.original).ok()?;
    let i = row.data.intent;
    let observation_expires = evidence.budget.at.checked_add(Duration::seconds(30))?;
    let until = std::cmp::min(q.expires, observation_expires);
    let current = Current {
        task: &q.task,
        proposal: &q.proposal,
        lane: &q.lane,
        caller: &q.actor,
        mode: &q.mode,
        mode_name: "present",
        mode_effective_at: q.mode_effective,
        mode_transition_pending: false,
        task_state: "live",
        proposal_state: "live",
        key_revoked: false,
        profile_qualified: true,
        profile_id: &q.profile_id,
        profile_write_paths: &q.profile_paths,
        model: &q.model,
        effort: &q.effort,
        tier: &q.tier,
        canonical_budget_revision: Some(&evidence.budget_record_sha256),
        observation_ref: Some(&evidence.budget.source_sha),
        exclusive_binding_ref: Some(&q.exclusive_evidence),
        coverage_established: true,
        coverage_objective: &i.budget.objective_id,
        coverage_envelope_sha: &evidence.task.sha256,
        coverage_valid_until: until,
        coverage_scope: "all_objective_actors",
    };
    current::check(
        &evidence.original,
        &evidence.answer,
        &evidence.board,
        &evidence.assertion.as_assertion(),
        &evidence.verified,
        &current,
        now,
    )
}
/// Recollect protected sources and atomically consume the exact decision and
/// create the local capacity claim/outbox. No native effect happens here.
#[allow(dead_code)]
pub(crate) fn admit_qualified(
    config: &Sources,
    db: &mut Connection,
    request: &str,
    q: &QualifiedContext,
    now: OffsetDateTime,
) -> Result<Admission, Held> {
    let parsed: crate::request::AdmissionRequest =
        serde_json::from_str(request).map_err(|_| Held("invalid_request"))?;
    if let Some(existing) = store::retrieve_existing(db, &parsed, &config.actor)
        .map_err(|_| Held("admission_conflict"))?
    {
        return Ok(existing.admission);
    }
    let evidence = inspect::collect(config, db, request, now)?;
    let gate = gate(config, &evidence, q, now).ok_or(Held("qualification_unavailable"))?;
    store::commit(
        db,
        &parsed.request_id,
        &evidence.original,
        &evidence.answer,
        &evidence.verified,
        &gate,
    )
    .map_err(|_| Held("admission_unavailable"))
}
/// Recollect sources before crossing the native boundary, bind the exact
/// already-admitted operation, and durably fence it as unknown before any
/// external call. Actual bridge delivery is intentionally not implemented.
#[allow(dead_code)]
pub(crate) fn prepare_delivery_qualified(
    config: &Sources,
    db: &mut Connection,
    request: &str,
    q: &QualifiedContext,
    operation: &str,
    now: OffsetDateTime,
) -> Result<(), Held> {
    let parsed: crate::request::AdmissionRequest =
        serde_json::from_str(request).map_err(|_| Held("invalid_request"))?;
    let evidence = inspect::collect(config, db, request, now)?;
    gate(config, &evidence, q, now).ok_or(Held("qualification_unavailable"))?;
    let existing = store::lookup_exact(
        db,
        &parsed.request_id,
        &evidence.task,
        &evidence.decision,
        &evidence.answer,
        &evidence.verified.assertion_digest,
    )
    .map_err(|_| Held("admission_unavailable"))?
    .ok_or(Held("admission_unavailable"))?;
    if existing.operation != operation {
        return Err(Held("operation_mismatch"));
    }
    store::fence_delivery(db, operation).map_err(|_| Held("delivery_fenced"))
}
/// Synthetic qualification is compiled only into tests; it is not a runtime
/// constructor and never certifies an OS grant or controller coverage.
#[cfg(test)]
pub(crate) fn synthetic(original: &str, actor: &str, expires: OffsetDateTime) -> QualifiedContext {
    let task = crate::validate_review(original).unwrap();
    let row: Row = serde_json::from_str(original).unwrap();
    let i = row.data.intent;
    QualifiedContext {
        task,
        proposal: i.proposal,
        mode: i.authority.mode,
        lane: i.lane_id,
        actor: actor.into(),
        mode_effective: time::macros::datetime!(2026-11-02 00:00 UTC),
        expires,
        profile_id: i.workspace.profile_id,
        profile_paths: i.effects.write_paths,
        model: i.model.id,
        effort: i.model.effort,
        tier: i.model.tier,
        exclusive_evidence: "test-only-exclusive".into(),
    }
}
