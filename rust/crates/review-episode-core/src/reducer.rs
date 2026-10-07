//! Deterministic reducer. The caller supplies the current durable state and
//! performs the eventual compare-and-swap; this module has no I/O or grants.

use crate::codec::{JsString, JsValue, digest, field, string_is};
use crate::command::{Action, BeginCommand, TransitionCommand};
use crate::identity::{Authority, Revision, validate_reference};
use crate::implementation_review_v1::{ReviewResult, Verdict, preserve_finding_lineage};
use crate::state::{Phase, ReviewEpisodeState, Status, validate_admission, validate_text_array};
use crate::{EpisodeError, EpisodeResult};

#[derive(Clone, Debug)]
pub enum TransitionOutcome {
    Applied(ReviewEpisodeState),
    Replay(ReviewEpisodeState),
}

impl TransitionOutcome {
    pub fn state(&self) -> &ReviewEpisodeState {
        match self {
            Self::Applied(state) | Self::Replay(state) => state,
        }
    }

    pub fn into_state(self) -> ReviewEpisodeState {
        match self {
            Self::Applied(state) | Self::Replay(state) => state,
        }
    }

    pub fn is_replay(&self) -> bool {
        matches!(self, Self::Replay(_))
    }
}

pub fn begin(
    current: Option<&ReviewEpisodeState>,
    command: &BeginCommand,
) -> EpisodeResult<TransitionOutcome> {
    if command.authority.writer.generation != 1 {
        return Err(EpisodeError::authority(
            "review episode initial writer generation is invalid",
        ));
    }
    let questions = JsValue::Array(command.unresolved_questions.clone());
    validate_text_array(&questions, "review episode unresolvedQuestions")?;
    let content = JsValue::object([
        ("action", JsValue::text("begin")),
        ("unresolvedQuestions", questions.clone()),
    ]);
    let content_revision = Revision(digest(&content));
    if let Some(existing) = current {
        if existing.identity != command.authority.identity {
            return Err(EpisodeError::authority(
                "review episode identity does not match current state",
            ));
        }
        if let Some(previous) = existing.handled(&command.transition_id)
            && previous == &content_revision.value()
        {
            authorize_current(existing, &command.authority)?;
            return Ok(TransitionOutcome::Replay(existing.clone()));
        }
        return Err(EpisodeError::new("review episode already exists"));
    }
    let transitions = JsValue::Object(
        std::iter::once((command.transition_id.clone(), content_revision.value())).collect(),
    );
    let semantic = JsValue::object([
        ("schemaVersion", JsValue::Number(1.0)),
        ("identity", command.authority.identity.value()),
        ("authority", command.authority.binding()),
        ("writer", command.authority.writer_value()),
        ("status", JsValue::text("active")),
        ("phase", JsValue::text("initial_review")),
        ("subject", command.authority.initial_subject.clone()),
        ("currentResult", JsValue::Null),
        ("unresolvedQuestions", questions),
        ("pendingAction", JsValue::text("perform_initial_review")),
        ("handledTransitions", transitions),
        ("continuity", JsValue::text("fresh_initial")),
        ("uncertainty", JsValue::Null),
        ("retirement", JsValue::Null),
    ]);
    Ok(TransitionOutcome::Applied(ReviewEpisodeState::publish(
        semantic,
    )?))
}

pub fn resume_initial(
    current: &ReviewEpisodeState,
    authority: &Authority,
) -> EpisodeResult<ReviewEpisodeState> {
    authorize_current(current, authority)?;
    if current.status != Status::Active
        || current.phase != Phase::InitialReview
        || current.get("currentResult") != &JsValue::Null
    {
        return Err(EpisodeError::new(
            "review episode is not awaiting its initial result",
        ));
    }
    Ok(current.clone())
}

pub fn validate_result(
    current: &ReviewEpisodeState,
    authority: &Authority,
    expected_revision: &Revision,
    result: JsValue,
) -> EpisodeResult<ReviewResult> {
    if &current.revision != expected_revision {
        return Err(EpisodeError::revision_conflict(
            "review episode expected revision does not match current state",
        ));
    }
    authorize_current(current, authority)?;
    if current.status != Status::Active
        || !matches!(current.phase, Phase::InitialReview | Phase::ReEvaluation)
    {
        return Err(EpisodeError::new(
            "review episode result transition is invalid",
        ));
    }
    let admitted = ReviewResult::parse(result)?;
    let subject = Revision::parse(
        field(current.get("subject"), "sha256")?,
        "review episode subject.sha256",
    )?;
    if admitted.subject_digest()? != subject {
        return Err(EpisodeError::result_contract(
            "review episode result does not match exact subject",
        ));
    }
    if let Some(previous) = current.current_result()? {
        preserve_finding_lineage(&previous.findings, &admitted.findings)?;
    }
    Ok(admitted)
}

pub fn transition(
    current: &ReviewEpisodeState,
    command: &TransitionCommand,
) -> EpisodeResult<TransitionOutcome> {
    if current.identity != command.authority.identity {
        return Err(EpisodeError::authority(
            "review episode identity does not match current state",
        ));
    }
    let content_revision = Revision(digest(&command.content()));
    if let Some(previous) = current.handled(&command.transition_id) {
        if previous != &content_revision.value() {
            return Err(EpisodeError::new(
                "review episode transition identity conflicts with durable content",
            ));
        }
        // R0-selected delta: even a matching command replay cannot disclose
        // current state to a replaced writer. Independent read authorization
        // is a host concern, not a property of this command.
        authorize_current(current, &command.authority)?;
        return Ok(TransitionOutcome::Replay(current.clone()));
    }
    if current.revision != command.expected_revision {
        return Err(EpisodeError::revision_conflict(
            "review episode expected revision does not match current state",
        ));
    }
    if current.status == Status::Retired {
        return Err(EpisodeError::new(
            "retired review episode cannot transition",
        ));
    }
    let mut next = current.semantic();
    if command.action == Action::ReplaceWriter {
        replace_writer(current, command, &mut next)?;
    } else {
        authorize_current(current, &command.authority)?;
        match command.action {
            Action::RecordResult => record_result(current, command, &mut next)?,
            Action::RecordRemediationSubject => {
                record_remediation_subject(current, command, &mut next)?
            }
            Action::SucceedEvidence => succeed_evidence(current, command, &mut next)?,
            Action::MarkUncertain => {
                field(&command.payload, "reason")?.nonempty_text("uncertainty.reason")?;
                field(&command.payload, "reconciliationAction")?
                    .nonempty_text("uncertainty.reconciliationAction")?;
                set(&mut next, "status", JsValue::text("uncertain"));
                set(&mut next, "uncertainty", command.payload.clone());
                set(
                    &mut next,
                    "pendingAction",
                    field(&command.payload, "reconciliationAction")?.clone(),
                );
            }
            Action::Retire => {
                set(&mut next, "status", JsValue::text("retired"));
                set(&mut next, "uncertainty", JsValue::Null);
                set(&mut next, "retirement", command.payload.clone());
                set(
                    &mut next,
                    "pendingAction",
                    JsValue::text("none_review_episode_retired"),
                );
            }
            Action::ReplaceWriter => unreachable!(),
        }
    }
    let mut handled = current.get("handledTransitions").as_object()?.clone();
    handled.insert(command.transition_id.clone(), content_revision.value());
    set(&mut next, "handledTransitions", JsValue::Object(handled));
    Ok(TransitionOutcome::Applied(ReviewEpisodeState::publish(
        next,
    )?))
}

fn authorize_current(current: &ReviewEpisodeState, authority: &Authority) -> EpisodeResult<()> {
    if current.identity != authority.identity
        || digest(current.get("authority")) != digest(&authority.binding())
        || digest(current.get("writer")) != digest(&authority.writer_value())
    {
        return Err(EpisodeError::authority(
            "review episode authority does not match current writer generation",
        ));
    }
    Ok(())
}

fn replace_writer(
    current: &ReviewEpisodeState,
    command: &TransitionCommand,
    next: &mut JsValue,
) -> EpisodeResult<()> {
    let successor = &command.authority;
    let current_subject = field(current.get("subject"), "sha256")?;
    let successor_subject = field(&successor.initial_subject, "sha256")?;
    if successor.writer.generation != current.writer.generation + 1
        || successor.predecessor_revision.as_ref() != Some(&current.revision)
        || successor_subject != current_subject
    {
        return Err(EpisodeError::authority(
            "review episode successor authority is invalid",
        ));
    }
    field(&command.payload, "reason")?.nonempty_text("replacement.reason")?;
    field(&command.payload, "pendingAction")?.nonempty_text("replacement.pendingAction")?;
    set(next, "authority", successor.binding());
    set(next, "writer", successor.writer_value());
    set(next, "status", JsValue::text("active"));
    set(next, "uncertainty", JsValue::Null);
    set(
        next,
        "pendingAction",
        field(&command.payload, "pendingAction")?.clone(),
    );
    set(
        next,
        "continuity",
        JsValue::text("reconstructed_continuation"),
    );
    Ok(())
}

fn record_result(
    current: &ReviewEpisodeState,
    command: &TransitionCommand,
    next: &mut JsValue,
) -> EpisodeResult<()> {
    let result = validate_result(
        current,
        &command.authority,
        &command.expected_revision,
        field(&command.payload, "result")?.clone(),
    )?;
    let questions = field(&command.payload, "unresolvedQuestions")?;
    validate_text_array(questions, "review episode unresolvedQuestions")?;
    let supplied = command.payload.get("evidenceAdmissions");
    let admissions = match supplied {
        Some(JsValue::Null) | None => &[],
        Some(value) => value.as_array()?,
    };
    for admission in admissions {
        validate_admission(admission)?;
    }
    let old_admissions = if current.schema_version() == 2 {
        current.get("evidenceAdmissions").as_array()?
    } else {
        &[]
    };
    let stale_required_admissions = !old_admissions.is_empty() && admissions.is_empty();
    let evidence_blocked = stale_required_admissions
        || admissions.iter().any(|admission| {
            !string_is(
                field(admission, "status").expect("validated admission"),
                "established",
            )
        });
    let pending = result.verdict != Verdict::AcceptableAsIs || !questions.as_array()?.is_empty();
    if !admissions.is_empty() {
        set(next, "schemaVersion", JsValue::Number(2.0));
        set(
            next,
            "evidenceAdmissions",
            JsValue::Array(admissions.to_vec()),
        );
    }
    set(next, "currentResult", result.value);
    set(next, "unresolvedQuestions", questions.clone());
    set(
        next,
        "phase",
        JsValue::text(if evidence_blocked {
            "evidence_unestablished"
        } else if pending {
            "remediation"
        } else {
            "reported"
        }),
    );
    set(
        next,
        "pendingAction",
        JsValue::text(if evidence_blocked {
            "route_unestablished_claim_to_owner"
        } else if pending {
            "await_remediation"
        } else {
            "return_review_result_to_builder"
        }),
    );
    set(next, "continuity", JsValue::text("same_session"));
    Ok(())
}

fn record_remediation_subject(
    current: &ReviewEpisodeState,
    command: &TransitionCommand,
    next: &mut JsValue,
) -> EpisodeResult<()> {
    if current.status != Status::Active
        || !matches!(current.phase, Phase::Remediation | Phase::Reported)
    {
        return Err(EpisodeError::new(
            "review episode remediation subject transition is invalid",
        ));
    }
    let subject = field(&command.payload, "subject")?;
    validate_reference(subject, "review episode remediation subject")?;
    set(next, "subject", subject.clone());
    set(next, "phase", JsValue::text("re_evaluation"));
    set(
        next,
        "pendingAction",
        JsValue::text("re_evaluate_delta_in_retained_session"),
    );
    set(next, "continuity", JsValue::text("same_session"));
    Ok(())
}

fn succeed_evidence(
    current: &ReviewEpisodeState,
    command: &TransitionCommand,
    next: &mut JsValue,
) -> EpisodeResult<()> {
    if current.status != Status::Active
        || current.phase != Phase::EvidenceUnestablished
        || current.schema_version() != 2
        || current.current_result()?.is_none()
    {
        return Err(EpisodeError::new(
            "review episode evidence succession requires blocked immutable review evidence",
        ));
    }
    let predecessor = field(&command.payload, "predecessorAdmissions")?.as_array()?;
    let successor = field(&command.payload, "successorAdmissions")?.as_array()?;
    if predecessor.len() != 2
        || successor.len() != 2
        || digest(&JsValue::Array(predecessor.to_vec()))
            != digest(current.get("evidenceAdmissions"))
    {
        return Err(EpisodeError::new(
            "review episode evidence succession predecessor differs from current admissions",
        ));
    }
    for admission in successor {
        validate_admission(admission)?;
        if !string_is(field(admission, "status")?, "established") {
            return Err(EpisodeError::new(
                "review episode successor evidence is not established",
            ));
        }
    }
    let mut admissions = current.get("evidenceAdmissions").as_array()?.to_vec();
    admissions.extend_from_slice(successor);
    set(next, "evidenceAdmissions", JsValue::Array(admissions));
    let result = current.current_result()?.expect("checked result");
    let has_questions = !current.get("unresolvedQuestions").as_array()?.is_empty();
    let pending = result.verdict != Verdict::AcceptableAsIs || has_questions;
    set(
        next,
        "phase",
        JsValue::text(if pending { "remediation" } else { "reported" }),
    );
    set(
        next,
        "pendingAction",
        JsValue::text(if pending {
            "await_remediation"
        } else {
            "return_review_result_to_builder"
        }),
    );
    set(next, "continuity", JsValue::text("same_session"));
    Ok(())
}

fn set(target: &mut JsValue, key: &str, value: JsValue) {
    target.as_object_mut().insert(JsString::new(key), value);
}

trait MutableObject {
    fn as_object_mut(&mut self) -> &mut std::collections::BTreeMap<JsString, JsValue>;
}

impl MutableObject for JsValue {
    fn as_object_mut(&mut self) -> &mut std::collections::BTreeMap<JsString, JsValue> {
        match self {
            JsValue::Object(map) => map,
            _ => unreachable!("validated semantic object"),
        }
    }
}
