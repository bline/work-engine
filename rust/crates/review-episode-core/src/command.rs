use crate::codec::{JsString, JsValue};
use crate::identity::{Authority, Revision};
use crate::{EpisodeError, EpisodeResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    RecordResult,
    RecordRemediationSubject,
    SucceedEvidence,
    MarkUncertain,
    Retire,
    ReplaceWriter,
}

impl Action {
    pub fn parse(value: &str) -> EpisodeResult<Self> {
        match value {
            "record_result" => Ok(Self::RecordResult),
            "record_remediation_subject" => Ok(Self::RecordRemediationSubject),
            "succeed_evidence" => Ok(Self::SucceedEvidence),
            "mark_uncertain" => Ok(Self::MarkUncertain),
            "retire" => Ok(Self::Retire),
            "replace_writer" => Ok(Self::ReplaceWriter),
            _ => Err(EpisodeError::new("review episode action is unsupported")),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::RecordResult => "record_result",
            Self::RecordRemediationSubject => "record_remediation_subject",
            Self::SucceedEvidence => "succeed_evidence",
            Self::MarkUncertain => "mark_uncertain",
            Self::Retire => "retire",
            Self::ReplaceWriter => "replace_writer",
        }
    }
}

#[derive(Clone, Debug)]
pub struct BeginCommand {
    pub(crate) authority: Authority,
    pub(crate) transition_id: JsString,
    pub(crate) unresolved_questions: Vec<JsValue>,
}

impl BeginCommand {
    pub fn new(
        authority: Authority,
        transition_id: &str,
        unresolved_questions: Vec<JsValue>,
    ) -> EpisodeResult<Self> {
        Self::new_utf16(
            authority,
            JsString::new(transition_id),
            unresolved_questions,
        )
    }

    pub fn new_utf16(
        authority: Authority,
        transition_id: JsString,
        unresolved_questions: Vec<JsValue>,
    ) -> EpisodeResult<Self> {
        if !transition_id.is_nonempty_text() {
            return Err(EpisodeError::new(
                "review episode transitionId must be non-empty text",
            ));
        }
        if unresolved_questions.iter().any(|item| {
            item.nonempty_text("review episode unresolvedQuestions")
                .is_err()
        }) {
            return Err(EpisodeError::new(
                "review episode unresolvedQuestions are invalid",
            ));
        }
        Ok(Self {
            authority,
            transition_id,
            unresolved_questions,
        })
    }
}

#[derive(Clone, Debug)]
pub struct TransitionCommand {
    pub(crate) authority: Authority,
    pub(crate) expected_revision: Revision,
    pub(crate) transition_id: JsString,
    pub(crate) action: Action,
    pub(crate) payload: JsValue,
}

impl TransitionCommand {
    pub fn new(
        authority: Authority,
        expected_revision: Revision,
        transition_id: &str,
        action: Action,
        payload: JsValue,
    ) -> EpisodeResult<Self> {
        Self::new_utf16(
            authority,
            expected_revision,
            JsString::new(transition_id),
            action,
            payload,
        )
    }

    pub fn new_utf16(
        authority: Authority,
        expected_revision: Revision,
        transition_id: JsString,
        action: Action,
        payload: JsValue,
    ) -> EpisodeResult<Self> {
        if !transition_id.is_nonempty_text() {
            return Err(EpisodeError::new(
                "review episode transitionId must be non-empty text",
            ));
        }
        payload.as_object()?;
        Ok(Self {
            authority,
            expected_revision,
            transition_id,
            action,
            payload,
        })
    }

    pub fn content(&self) -> JsValue {
        JsValue::object([
            ("action", JsValue::text(self.action.name())),
            ("payload", self.payload.clone()),
        ])
    }
}
