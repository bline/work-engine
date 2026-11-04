//! Internal eligibility kernel. No public constructor or source adapter is
//! exposed: all context must come from independently protected owners.
use crate::{
    ModeView, Ref, Row,
    board::{CompleteBoard, own_close},
    hash,
    proof::{Assertion, VerifiedAnswer},
    store::CurrentGate,
    validate_review,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[allow(dead_code)]
pub(crate) struct Current<'a> {
    pub task: &'a Ref,
    pub proposal: &'a Ref,
    pub lane: &'a str,
    pub caller: &'a str,
    pub mode: &'a Ref,
    pub mode_name: &'a str,
    pub mode_effective_at: OffsetDateTime,
    pub mode_transition_pending: bool,
    pub task_state: &'a str,
    pub proposal_state: &'a str,
    pub key_revoked: bool,
    pub profile_qualified: bool,
    pub profile_id: &'a str,
    pub profile_write_paths: &'a [String],
    pub model: &'a str,
    pub effort: &'a str,
    pub tier: &'a str,
    pub canonical_budget_revision: Option<&'a str>,
    pub observation_ref: Option<&'a str>,
    pub exclusive_binding_ref: Option<&'a str>,
    pub coverage_established: bool,
    pub coverage_objective: &'a str,
    pub coverage_envelope_sha: &'a str,
    pub coverage_valid_until: OffsetDateTime,
    pub coverage_scope: &'a str,
}
/// Each input is an observation obtained from a protected owner, not an MCP
/// request. This module is intentionally unreachable until those owners exist.
#[allow(dead_code)]
pub(crate) fn check(
    raw_request: &str,
    raw_answer: &str,
    board: &CompleteBoard,
    assertion: &Assertion<'_>,
    verified: &VerifiedAnswer,
    current: &Current<'_>,
    now: OffsetDateTime,
) -> Option<CurrentGate> {
    let task = validate_review(raw_request).ok()?;
    let row: Row = serde_json::from_str(raw_request).ok()?;
    let i = row.data.intent;
    let mode_artifact = row
        .data
        .intent_artifacts
        .iter()
        .find(|a| a.r#ref == i.authority.mode)?;
    let signed_mode: ModeView = serde_json::from_str(&mode_artifact.content_json).ok()?;
    let mode_at = OffsetDateTime::parse(&signed_mode.effective_at, &Rfc3339).ok()?;
    let admit = OffsetDateTime::parse(&i.clock.admit_until, &Rfc3339).ok()?;
    let close = OffsetDateTime::parse(&i.clock.close_by, &Rfc3339).ok()?;
    let reply = own_close(board, raw_request, raw_answer, assertion, verified)?;
    let reply_row: serde_json::Value = serde_json::from_str(reply).ok()?;
    let decision = Ref {
        id: verified.answer_id.clone(),
        revision: reply_row.get("sequence")?.as_u64()?.to_string(),
        sha256: hash(reply),
    };
    let budget_revision = current
        .canonical_budget_revision
        .filter(|s| !s.is_empty())?;
    let observation = current.observation_ref.filter(|s| !s.is_empty())?;
    let exclusive = current.exclusive_binding_ref.filter(|s| !s.is_empty())?;
    if &task != current.task
        || &i.proposal != current.proposal
        || i.lane_id != current.lane
        || i.submitter != current.caller
        || &i.authority.mode != current.mode
        || current.mode_name != "present"
        || current.mode_effective_at != mode_at
        || mode_at > now
        || current.mode_transition_pending
        || current.task_state != "live"
        || current.proposal_state != "live"
        || current.key_revoked
        || !current.profile_qualified
        || i.workspace.profile_id != current.profile_id
        || i.effects
            .write_paths
            .iter()
            .any(|path| !current.profile_write_paths.contains(path))
        || i.model.id != current.model
        || i.model.effort != current.effort
        || i.model.tier != current.tier
        || now >= admit
        || close <= admit
        || now >= verified.expires_at
        || !current.coverage_established
        || current.coverage_objective != i.budget.objective_id
        || current.coverage_envelope_sha != task.sha256
        || now >= current.coverage_valid_until
        || current.coverage_scope != "all_objective_actors"
        || verified.request_digest != hash(raw_request)
    {
        return None;
    }
    Some(CurrentGate {
        task_id: task.id,
        revision: task.revision,
        request_sha: task.sha256,
        lane_id: i.lane_id,
        proposal: i.proposal,
        decision,
        objective_id: i.budget.objective_id,
        canonical_budget_revision: budget_revision.to_owned(),
        observation_ref: observation.to_owned(),
        exclusive_binding_ref: exclusive.to_owned(),
        maximum: i.budget.maximum,
        productive_maximum: i.budget.productive_maximum,
        closure_reserve: i.budget.closure_reserve,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use time::macros::datetime;
    #[test]
    fn current_state_revocation_actor_and_deadline_fence() {
        let snapshot: Value =
            serde_json::from_str(include_str!("../tests/projection-fixture.json")).unwrap();
        let raw = snapshot["tasks"][0]["request_json"].as_str().unwrap();
        let row: Row = serde_json::from_str(raw).unwrap();
        let i = &row.data.intent;
        let task = validate_review(raw).unwrap();
        let answer = "synthetic answer";
        let proof = VerifiedAnswer {
            decision: "approve".into(),
            comment: String::new(),
            answer_id: "a".into(),
            answer_digest: hash(answer),
            assertion_digest: super::super::proof::assertion_digest(&Assertion {
                credential_id: "c",
                authenticator_data: "d",
                client_data_json: "j",
                signature: "s",
            }),
            board_seq: 9001,
            request_digest: hash(raw),
            expires_at: datetime!(2026-11-03 00:00 UTC),
        };
        let assertion = Assertion {
            credential_id: "c",
            authenticator_data: "d",
            client_data_json: "j",
            signature: "s",
        };
        let reply=json!({"kind":"reply","sequence":9002,"in_reply_to":9001,"topic":"fixture:task","to":["session:fixture-session"],"message_id":"a","data":{"disposition":"answered","decision":"approve","comment":"","answer_id":"a","signed_answer":{"format":"work-engine.owner-answer/v1","answer_json":answer,"credential_id":"c","authenticator_data":"d","client_data_json":"j","signature":"s"}}}).to_string();
        let board = CompleteBoard {
            revision: "r1".into(),
            rows: vec![raw.to_owned(), reply],
        };
        let mut current = Current {
            task: &task,
            proposal: &i.proposal,
            lane: &i.lane_id,
            caller: &i.submitter,
            mode: &i.authority.mode,
            mode_name: "present",
            mode_effective_at: datetime!(2026-11-02 00:00 UTC),
            mode_transition_pending: false,
            task_state: "live",
            proposal_state: "live",
            key_revoked: false,
            profile_qualified: true,
            profile_id: &i.workspace.profile_id,
            profile_write_paths: &i.effects.write_paths,
            model: &i.model.id,
            effort: &i.model.effort,
            tier: &i.model.tier,
            canonical_budget_revision: Some("budget-r1"),
            observation_ref: Some("observation-r1"),
            exclusive_binding_ref: Some("exclusive-binding-r1"),
            coverage_established: true,
            coverage_objective: &i.budget.objective_id,
            coverage_envelope_sha: &task.sha256,
            coverage_valid_until: datetime!(2026-11-03 01:00 UTC),
            coverage_scope: "all_objective_actors",
        };
        let now = datetime!(2026-11-02 01:00 UTC);
        assert!(check(raw, answer, &board, &assertion, &proof, &current, now).is_some());
        current.key_revoked = true;
        assert!(check(raw, answer, &board, &assertion, &proof, &current, now).is_none());
        current.key_revoked = false;
        current.caller = "other";
        assert!(check(raw, answer, &board, &assertion, &proof, &current, now).is_none());
        current.caller = &i.submitter;
        current.coverage_established = false;
        assert!(check(raw, answer, &board, &assertion, &proof, &current, now).is_none());
        current.coverage_established = true;
        current.task_state = "withdrawn";
        assert!(check(raw, answer, &board, &assertion, &proof, &current, now).is_none());
        current.task_state = "live";
        current.mode_transition_pending = true;
        assert!(check(raw, answer, &board, &assertion, &proof, &current, now).is_none());
        current.mode_transition_pending = false;
        current.coverage_envelope_sha = "wrong";
        assert!(check(raw, answer, &board, &assertion, &proof, &current, now).is_none());
        current.coverage_envelope_sha = &task.sha256;
        current.proposal_state = "superseded";
        assert!(check(raw, answer, &board, &assertion, &proof, &current, now).is_none());
        current.proposal_state = "live";
        assert!(
            check(
                raw,
                answer,
                &board,
                &assertion,
                &proof,
                &current,
                datetime!(2026-11-03 00:00 UTC)
            )
            .is_none()
        );
    }
}
