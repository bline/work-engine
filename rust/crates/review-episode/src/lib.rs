//! Offline review-episode application. Production host admission is deliberately absent.
pub mod admission;
pub mod protocol;

use review_episode_core::codec::{JsValue, exact_fields, field};
use review_episode_core::command::{Action, BeginCommand, TransitionCommand};
use review_episode_core::identity::{Authority, Identity, Revision};
use review_episode_core::reducer::{self, TransitionOutcome};
use review_episode_core::{EpisodeError, ErrorKind};
use review_episode_store::{EpisodeStore, StoreError, WriteDisposition};
use thiserror::Error;

use admission::HostAdmissionPort;
use protocol::{Request, reply};

#[derive(Debug, Error)]
pub enum AppError {
    #[error("request framing error: {0}")]
    Framing(String),
    #[error("offline fixture admission failed: {0}")]
    Admission(String),
    #[error("response exceeds configured byte limit")]
    ResponseTooLarge,
    #[error("request I/O failed: {0}")]
    Io(String),
    #[error("{0}")]
    Domain(EpisodeError),
    #[error("{0}")]
    Store(#[from] StoreError),
}
impl From<EpisodeError> for AppError {
    fn from(e: EpisodeError) -> Self {
        Self::Domain(e)
    }
}
pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Framing(_) => "Framing",
            Self::Admission(_) => "Admission",
            Self::ResponseTooLarge => "ResponseTooLarge",
            Self::Io(_) => "Io",
            Self::Domain(e) => match e.kind {
                ErrorKind::InvalidCommand => "InvalidCommand",
                ErrorKind::ResultContract => "ResultContract",
                ErrorKind::Authority => "Authority",
                ErrorKind::RevisionConflict => "RevisionConflict",
                ErrorKind::StateIntegrity => "StateIntegrity",
            },
            Self::Store(e) => match e {
                StoreError::Path(_) => "Path",
                StoreError::Schema(_) => "Schema",
                StoreError::Integrity(_) => "StateIntegrity",
                StoreError::RevisionConflict => "RevisionConflict",
                StoreError::Busy => "Busy",
                StoreError::ResponseTooLarge => "ResponseTooLarge",
                StoreError::OutcomeUnknown => "OutcomeUnknown",
                StoreError::Io(_) => "Io",
                StoreError::Domain(e) => match e.kind {
                    ErrorKind::InvalidCommand => "InvalidCommand",
                    ErrorKind::ResultContract => "ResultContract",
                    ErrorKind::Authority => "Authority",
                    ErrorKind::RevisionConflict => "RevisionConflict",
                    ErrorKind::StateIntegrity => "StateIntegrity",
                },
            },
        }
    }
    pub fn public_message(&self) -> String {
        match self {
            Self::Domain(error) | Self::Store(StoreError::Domain(error)) => {
                error.message.chars().take(160).collect()
            }
            _ => format!("{} offline request failure", self.kind()),
        }
    }
}

pub struct Application<P: HostAdmissionPort> {
    pub store: EpisodeStore,
    pub port: P,
    pub response_limit: usize,
}

impl<P: HostAdmissionPort> Application<P> {
    pub fn execute(&mut self, request: &Request) -> AppResult<String> {
        match request.operation.as_str() {
            "begin" => self.begin(request),
            "transition" => self.transition(request),
            "resumeInitial" => self.resume_initial(request),
            "validateResult" => self.validate_result(request),
            "recover" => self.recover(request),
            "read" => self.read(request),
            "history" => self.history(request),
            _ => Err(AppError::Framing("unsupported operation".into())),
        }
    }

    fn begin(&mut self, request: &Request) -> AppResult<String> {
        exact_fields(
            &request.args,
            &["authority", "transitionId", "unresolvedQuestions"],
            "begin arguments",
        )?;
        let authority = Authority::parse(field(&request.args, "authority")?.clone())?;
        let id = field(&request.args, "transitionId")?
            .nonempty_text("transitionId")?
            .clone();
        let questions = field(&request.args, "unresolvedQuestions")?
            .as_array()?
            .to_vec();
        let key = authority.identity().key().0;
        let observed = self
            .store
            .read(&key)?
            .current
            .map(|s| s.revision().0.clone());
        self.port.admit(request, observed.as_deref())?;
        review_episode_store::test_checkpoint("after_admission");
        let command = BeginCommand::new_utf16(authority, id, questions)?;
        let result =
            self.store
                .write(&key, observed.as_deref(), self.response_limit, |current| {
                    let outcome = reducer::begin(current, &command)?;
                    Ok(write_outcome(request, outcome))
                })?;
        Ok(result)
    }

    fn transition(&mut self, request: &Request) -> AppResult<String> {
        exact_fields(
            &request.args,
            &[
                "authority",
                "expectedRevision",
                "transitionId",
                "action",
                "payload",
            ],
            "transition arguments",
        )?;
        let authority = Authority::parse(field(&request.args, "authority")?.clone())?;
        let expected = Revision::parse(
            field(&request.args, "expectedRevision")?,
            "expectedRevision",
        )?;
        let id = field(&request.args, "transitionId")?
            .nonempty_text("transitionId")?
            .clone();
        let action = Action::parse(
            &field(&request.args, "action")?
                .as_text()?
                .to_string_checked()?,
        )?;
        let payload = field(&request.args, "payload")?.clone();
        let key = authority.identity().key().0;
        let observed = self
            .store
            .read(&key)?
            .current
            .map(|s| s.revision().0.clone());
        self.port.admit(request, observed.as_deref())?;
        review_episode_store::test_checkpoint("after_admission");
        let command = TransitionCommand::new_utf16(authority, expected, id, action, payload)?;
        let result =
            self.store
                .write(&key, observed.as_deref(), self.response_limit, |current| {
                    let current = current.ok_or_else(|| {
                        StoreError::Domain(EpisodeError::revision_conflict(
                            "review episode does not exist",
                        ))
                    })?;
                    let outcome = reducer::transition(current, &command)?;
                    Ok(write_outcome(request, outcome))
                })?;
        Ok(result)
    }

    fn resume_initial(&mut self, request: &Request) -> AppResult<String> {
        exact_fields(&request.args, &["authority"], "resumeInitial arguments")?;
        let authority = Authority::parse(field(&request.args, "authority")?.clone())?;
        let snapshot = self.store.read(&authority.identity().key().0)?;
        self.port.admit(
            request,
            snapshot.current.as_ref().map(|s| s.revision().0.as_str()),
        )?;
        let current = snapshot
            .current
            .ok_or_else(|| EpisodeError::revision_conflict("review episode does not exist"))?;
        let state = reducer::resume_initial(&current, &authority)?;
        self.bounded(reply(
            request,
            "observed",
            vec![
                ("observedRevision", state.revision().value()),
                ("state", state.value().clone()),
            ],
        ))
    }

    fn validate_result(&mut self, request: &Request) -> AppResult<String> {
        exact_fields(
            &request.args,
            &["authority", "expectedRevision", "result"],
            "validateResult arguments",
        )?;
        let authority = Authority::parse(field(&request.args, "authority")?.clone())?;
        let expected = Revision::parse(
            field(&request.args, "expectedRevision")?,
            "expectedRevision",
        )?;
        let snapshot = self.store.read(&authority.identity().key().0)?;
        self.port.admit(
            request,
            snapshot.current.as_ref().map(|s| s.revision().0.as_str()),
        )?;
        let current = snapshot
            .current
            .ok_or_else(|| EpisodeError::revision_conflict("review episode does not exist"))?;
        let result = reducer::validate_result(
            &current,
            &authority,
            &expected,
            field(&request.args, "result")?.clone(),
        )?;
        self.bounded(reply(
            request,
            "observed",
            vec![
                ("observedRevision", current.revision().value()),
                ("result", result.value),
            ],
        ))
    }

    fn read_snapshot(
        &mut self,
        request: &Request,
    ) -> AppResult<(Identity, review_episode_store::Snapshot)> {
        let identity = Identity::parse(field(&request.args, "identity")?)?;
        let snapshot = self.store.read(&identity.key().0)?;
        self.port.admit(
            request,
            snapshot.current.as_ref().map(|s| s.revision().0.as_str()),
        )?;
        Ok((identity, snapshot))
    }

    fn read(&mut self, request: &Request) -> AppResult<String> {
        exact_fields(&request.args, &["identity", "revision"], "read arguments")?;
        let (_, snapshot) = self.read_snapshot(request)?;
        let revision = match field(&request.args, "revision")? {
            JsValue::Null => None,
            other => Some(Revision::parse(other, "revision")?),
        };
        let state = match revision {
            None => snapshot.current.as_ref(),
            Some(rev) => snapshot
                .history
                .iter()
                .find(|h| h.revision == rev.0)
                .map(|h| &h.state),
        };
        let observed = snapshot
            .current
            .as_ref()
            .map_or(JsValue::Null, |s| s.revision().value());
        self.bounded(reply(
            request,
            "observed",
            vec![
                ("observedRevision", observed),
                ("state", state.map_or(JsValue::Null, |s| s.value().clone())),
            ],
        ))
    }

    fn history(&mut self, request: &Request) -> AppResult<String> {
        exact_fields(&request.args, &["identity"], "history arguments")?;
        let (_, snapshot) = self.read_snapshot(request)?;
        let observed = snapshot
            .current
            .as_ref()
            .map_or(JsValue::Null, |s| s.revision().value());
        let states = snapshot
            .history
            .into_iter()
            .map(|h| h.state.value().clone())
            .collect();
        self.bounded(reply(
            request,
            "observed",
            vec![
                ("observedRevision", observed),
                ("history", JsValue::Array(states)),
            ],
        ))
    }

    fn recover(&mut self, request: &Request) -> AppResult<String> {
        exact_fields(
            &request.args,
            &["identity", "transitionId", "contentDigest"],
            "recover arguments",
        )?;
        let (_, snapshot) = self.read_snapshot(request)?;
        let id = field(&request.args, "transitionId")?
            .nonempty_text("transitionId")?
            .clone();
        let digest = Revision::parse(field(&request.args, "contentDigest")?, "contentDigest")?;
        let observed = snapshot
            .current
            .as_ref()
            .map_or(JsValue::Null, |s| s.revision().value());
        let mut introduced = None;
        for entry in &snapshot.history {
            if let Some(found) = entry.state.handled(&id) {
                if found != &digest.value() {
                    return Err(AppError::Domain(EpisodeError::new(
                        "transition identity conflicts with durable content",
                    )));
                }
                introduced = Some(entry.revision.clone());
                break;
            }
        }
        let extra = vec![
            ("observedRevision", observed),
            (
                "committedRevision",
                introduced
                    .as_ref()
                    .map_or(JsValue::Null, |r| JsValue::text(r)),
            ),
            (
                "state",
                snapshot
                    .current
                    .as_ref()
                    .map_or(JsValue::Null, |s| s.value().clone()),
            ),
        ];
        self.bounded(reply(
            request,
            if introduced.is_some() {
                "committed"
            } else {
                "absent"
            },
            extra,
        ))
    }

    fn bounded(&self, reply: String) -> AppResult<String> {
        if reply.len() > self.response_limit {
            Err(AppError::ResponseTooLarge)
        } else {
            Ok(reply)
        }
    }
}

fn write_outcome(request: &Request, outcome: TransitionOutcome) -> WriteDisposition {
    let replay = outcome.is_replay();
    let state = outcome.into_state();
    let reply_json = reply(
        request,
        if replay { "replay" } else { "applied" },
        vec![
            ("observedRevision", state.revision().value()),
            ("state", state.value().clone()),
        ],
    );
    if replay {
        WriteDisposition::Replay { reply_json }
    } else {
        WriteDisposition::Applied {
            state: Box::new(state),
            reply_json,
        }
    }
}
