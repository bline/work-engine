//! Typed operations for an in-process workflow. The same owner and admission
//! port are used by direct calls and by the legacy executable's wire adapter.
use review_episode_core::EpisodeError;
use review_episode_core::codec::{JsString, JsValue, exact_fields, field};
use review_episode_core::command::Action;
use review_episode_core::identity::{Authority, Identity, Revision};

use crate::protocol::Request;
use crate::{AppError, AppResult};

#[derive(Clone, Debug)]
pub enum Operation {
    Begin {
        authority: Authority,
        transition_id: JsString,
        unresolved_questions: Vec<JsValue>,
    },
    Transition {
        authority: Authority,
        expected_revision: Revision,
        transition_id: JsString,
        action: Action,
        payload: JsValue,
    },
    ResumeInitial {
        authority: Authority,
    },
    ValidateResult {
        authority: Authority,
        expected_revision: Revision,
        result: JsValue,
    },
    Recover {
        identity: Identity,
        transition: Option<(JsString, Revision)>,
    },
    Read {
        identity: Identity,
        revision: Option<Revision>,
    },
    History {
        identity: Identity,
    },
}

impl Operation {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Begin { .. } => "begin",
            Self::Transition { .. } => "transition",
            Self::ResumeInitial { .. } => "resumeInitial",
            Self::ValidateResult { .. } => "validateResult",
            Self::Recover { .. } => "recover",
            Self::Read { .. } => "read",
            Self::History { .. } => "history",
        }
    }

    pub(crate) fn args(&self) -> JsValue {
        match self {
            Self::Begin {
                authority,
                transition_id,
                unresolved_questions,
            } => JsValue::object([
                ("authority", authority.value().clone()),
                ("transitionId", JsValue::String(transition_id.clone())),
                (
                    "unresolvedQuestions",
                    JsValue::Array(unresolved_questions.clone()),
                ),
            ]),
            Self::Transition {
                authority,
                expected_revision,
                transition_id,
                action,
                payload,
            } => JsValue::object([
                ("authority", authority.value().clone()),
                ("expectedRevision", expected_revision.value()),
                ("transitionId", JsValue::String(transition_id.clone())),
                ("action", JsValue::text(action.name())),
                ("payload", payload.clone()),
            ]),
            Self::ResumeInitial { authority } => {
                JsValue::object([("authority", authority.value().clone())])
            }
            Self::ValidateResult {
                authority,
                expected_revision,
                result,
            } => JsValue::object([
                ("authority", authority.value().clone()),
                ("expectedRevision", expected_revision.value()),
                ("result", result.clone()),
            ]),
            Self::Recover {
                identity,
                transition: Some((id, digest)),
            } => JsValue::object([
                ("identity", identity.value()),
                ("transitionId", JsValue::String(id.clone())),
                ("contentDigest", digest.value()),
            ]),
            Self::Recover {
                identity,
                transition: None,
            } => JsValue::object([("identity", identity.value())]),
            Self::Read { identity, revision } => JsValue::object([
                ("identity", identity.value()),
                (
                    "revision",
                    revision.as_ref().map_or(JsValue::Null, Revision::value),
                ),
            ]),
            Self::History { identity } => JsValue::object([("identity", identity.value())]),
        }
    }

    /// Validate public typed fields against the same domain shapes accepted by
    /// the wire parser. This inspects in-memory values; it never JSON-encodes
    /// and re-parses an operation.
    pub(crate) fn validate(&self) -> AppResult<()> {
        let check_identity = |identity: &Identity| -> AppResult<()> {
            Identity::parse(&identity.value())?;
            Ok(())
        };
        let check_revision = |revision: &Revision, label: &str| -> AppResult<()> {
            Revision::parse(&revision.value(), label)?;
            Ok(())
        };
        let check_transition = |id: &JsString| -> AppResult<()> {
            JsValue::String(id.clone()).nonempty_text("transitionId")?;
            Ok(())
        };
        match self {
            Self::Begin {
                transition_id,
                unresolved_questions,
                ..
            } => {
                check_transition(transition_id)?;
                if unresolved_questions.iter().any(|item| {
                    item.nonempty_text("review episode unresolvedQuestions")
                        .is_err()
                }) {
                    return Err(EpisodeError::new(
                        "review episode unresolvedQuestions are invalid",
                    )
                    .into());
                }
            }
            Self::Transition {
                expected_revision,
                transition_id,
                payload,
                ..
            } => {
                check_revision(expected_revision, "expectedRevision")?;
                check_transition(transition_id)?;
                payload.as_object()?;
            }
            Self::ValidateResult {
                expected_revision, ..
            } => check_revision(expected_revision, "expectedRevision")?,
            Self::Recover {
                identity,
                transition,
            } => {
                check_identity(identity)?;
                if let Some((id, digest)) = transition {
                    check_transition(id)?;
                    check_revision(digest, "contentDigest")?;
                }
            }
            Self::Read { identity, revision } => {
                check_identity(identity)?;
                if let Some(revision) = revision {
                    check_revision(revision, "revision")?;
                }
            }
            Self::History { identity } => check_identity(identity)?,
            Self::ResumeInitial { .. } => {}
        }
        Ok(())
    }

    pub(crate) fn parse(request: &Request) -> AppResult<Self> {
        let a = &request.args;
        let operation = match request.operation.as_str() {
            "begin" => {
                exact_fields(
                    a,
                    &["authority", "transitionId", "unresolvedQuestions"],
                    "begin arguments",
                )?;
                Self::Begin {
                    authority: Authority::parse(field(a, "authority")?.clone())?,
                    transition_id: field(a, "transitionId")?
                        .nonempty_text("transitionId")?
                        .clone(),
                    unresolved_questions: field(a, "unresolvedQuestions")?.as_array()?.to_vec(),
                }
            }
            "transition" => {
                exact_fields(
                    a,
                    &[
                        "authority",
                        "expectedRevision",
                        "transitionId",
                        "action",
                        "payload",
                    ],
                    "transition arguments",
                )?;
                Self::Transition {
                    authority: Authority::parse(field(a, "authority")?.clone())?,
                    expected_revision: Revision::parse(
                        field(a, "expectedRevision")?,
                        "expectedRevision",
                    )?,
                    transition_id: field(a, "transitionId")?
                        .nonempty_text("transitionId")?
                        .clone(),
                    action: Action::parse(&field(a, "action")?.as_text()?.to_string_checked()?)?,
                    payload: field(a, "payload")?.clone(),
                }
            }
            "resumeInitial" => {
                exact_fields(a, &["authority"], "resumeInitial arguments")?;
                Self::ResumeInitial {
                    authority: Authority::parse(field(a, "authority")?.clone())?,
                }
            }
            "validateResult" => {
                exact_fields(
                    a,
                    &["authority", "expectedRevision", "result"],
                    "validateResult arguments",
                )?;
                Self::ValidateResult {
                    authority: Authority::parse(field(a, "authority")?.clone())?,
                    expected_revision: Revision::parse(
                        field(a, "expectedRevision")?,
                        "expectedRevision",
                    )?,
                    result: field(a, "result")?.clone(),
                }
            }
            "recover" => {
                let transition = if request.version() == 2 && a.as_object()?.len() == 1 {
                    exact_fields(a, &["identity"], "native recover arguments")?;
                    None
                } else {
                    exact_fields(
                        a,
                        &["identity", "transitionId", "contentDigest"],
                        "recover arguments",
                    )?;
                    Some((
                        field(a, "transitionId")?
                            .nonempty_text("transitionId")?
                            .clone(),
                        Revision::parse(field(a, "contentDigest")?, "contentDigest")?,
                    ))
                };
                Self::Recover {
                    identity: Identity::parse(field(a, "identity")?)?,
                    transition,
                }
            }
            "read" => {
                exact_fields(a, &["identity", "revision"], "read arguments")?;
                Self::Read {
                    identity: Identity::parse(field(a, "identity")?)?,
                    revision: match field(a, "revision")? {
                        JsValue::Null => None,
                        value => Some(Revision::parse(value, "revision")?),
                    },
                }
            }
            "history" => {
                exact_fields(a, &["identity"], "history arguments")?;
                Self::History {
                    identity: Identity::parse(field(a, "identity")?)?,
                }
            }
            _ => return Err(AppError::Framing("unsupported operation".into())),
        };
        operation.validate()?;
        Ok(operation)
    }
}
