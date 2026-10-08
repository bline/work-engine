//! Review-episode application with separate offline and native-host admission ports.
pub mod admission;
pub mod host_admission;
pub mod operation;
pub mod protocol;

use std::path::Path;

use review_episode_core::codec::{JsString, JsValue, digest};
use review_episode_core::command::{BeginCommand, TransitionCommand};
use review_episode_core::identity::{Identity, Revision};
use review_episode_core::reducer::{self, TransitionOutcome};
use review_episode_core::state::ReviewEpisodeState;
use review_episode_core::{EpisodeError, ErrorKind};
use review_episode_store::{EpisodeStore, Snapshot, StoreError, WriteDisposition, WriteResult};
use thiserror::Error;

use admission::HostAdmissionPort;
use operation::Operation;
use protocol::{DirectRequest, Request, reply};

#[derive(Clone, Debug)]
pub struct PossibleCommit {
    pub root: std::path::PathBuf,
    pub operation: &'static str,
    pub identity: Identity,
    pub transition_id: JsString,
    pub content_digest: Revision,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("request framing error: {0}")]
    Framing(String),
    #[error("review-episode admission failed: {0}")]
    Admission(String),
    #[error("response exceeds configured byte limit")]
    ResponseTooLarge,
    #[error("request I/O failed: {0}")]
    Io(String),
    #[error("{0}")]
    Domain(EpisodeError),
    #[error("review-episode operation outcome is unknown; reconcile exact transition")]
    OutcomeUnknown(Box<PossibleCommit>),
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
            Self::OutcomeUnknown(_) => "OutcomeUnknown",
            Self::Domain(e) => episode_kind(e),
            Self::Store(e) => match e {
                StoreError::Path(_) => "Path",
                StoreError::Schema(_) => "Schema",
                StoreError::Integrity(_) => "StateIntegrity",
                StoreError::RevisionConflict => "RevisionConflict",
                StoreError::Busy => "Busy",
                StoreError::ResponseTooLarge => "ResponseTooLarge",
                StoreError::OutcomeUnknown => "OutcomeUnknown",
                StoreError::Io(_) => "Io",
                StoreError::Domain(e) => episode_kind(e),
            },
        }
    }
    pub fn public_message(&self) -> String {
        match self {
            Self::Domain(error) | Self::Store(StoreError::Domain(error)) => {
                error.message.chars().take(160).collect()
            }
            _ => format!("{} review-episode request failure", self.kind()),
        }
    }
    pub fn possible_commit(&self) -> Option<&PossibleCommit> {
        match self {
            Self::OutcomeUnknown(context) => Some(context),
            _ => None,
        }
    }
}

fn episode_kind(e: &EpisodeError) -> &'static str {
    match e.kind {
        ErrorKind::InvalidCommand => "InvalidCommand",
        ErrorKind::ResultContract => "ResultContract",
        ErrorKind::Authority => "Authority",
        ErrorKind::RevisionConflict => "RevisionConflict",
        ErrorKind::StateIntegrity => "StateIntegrity",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteStatus {
    Applied,
    Replay,
}

#[derive(Debug)]
pub enum OperationResult {
    Write {
        status: WriteStatus,
        state: ReviewEpisodeState,
        context: PossibleCommit,
    },
    ObservedState {
        observed_revision: Option<Revision>,
        state: Option<ReviewEpisodeState>,
    },
    ValidatedResult {
        observed_revision: Revision,
        result: JsValue,
    },
    History {
        observed_revision: Option<Revision>,
        history: Vec<ReviewEpisodeState>,
    },
    Recovery {
        observed_revision: Option<Revision>,
        committed_revision: Option<Revision>,
        state: Option<ReviewEpisodeState>,
    },
}

impl OperationResult {
    pub fn wire_json(&self, request: &Request) -> String {
        let observed =
            |revision: &Option<Revision>| revision.as_ref().map_or(JsValue::Null, Revision::value);
        match self {
            Self::Write { status, state, .. } => reply(
                request,
                if *status == WriteStatus::Replay {
                    "replay"
                } else {
                    "applied"
                },
                vec![
                    ("observedRevision", state.revision().value()),
                    ("state", state.value().clone()),
                ],
            ),
            Self::ObservedState {
                observed_revision,
                state,
            } => reply(
                request,
                "observed",
                vec![
                    ("observedRevision", observed(observed_revision)),
                    (
                        "state",
                        state.as_ref().map_or(JsValue::Null, |s| s.value().clone()),
                    ),
                ],
            ),
            Self::ValidatedResult {
                observed_revision,
                result,
            } => reply(
                request,
                "observed",
                vec![
                    ("observedRevision", observed_revision.value()),
                    ("result", result.clone()),
                ],
            ),
            Self::History {
                observed_revision,
                history,
            } => reply(
                request,
                "observed",
                vec![
                    ("observedRevision", observed(observed_revision)),
                    (
                        "history",
                        JsValue::Array(history.iter().map(|s| s.value().clone()).collect()),
                    ),
                ],
            ),
            Self::Recovery {
                observed_revision,
                committed_revision,
                state,
            } => reply(
                request,
                if committed_revision.is_some() {
                    "committed"
                } else {
                    "absent"
                },
                vec![
                    ("observedRevision", observed(observed_revision)),
                    ("committedRevision", observed(committed_revision)),
                    (
                        "state",
                        state.as_ref().map_or(JsValue::Null, |s| s.value().clone()),
                    ),
                ],
            ),
        }
    }
    pub fn encoded_len(&self, request: &Request) -> usize {
        self.wire_json(request).len()
    }

    pub fn transition_context(&self) -> Option<&PossibleCommit> {
        match self {
            Self::Write { context, .. } => Some(context),
            _ => None,
        }
    }
}

pub struct Application<P: HostAdmissionPort> {
    store: EpisodeStore,
    port: P,
    response_limit: usize,
}

impl<P: HostAdmissionPort> Application<P> {
    pub fn new(store: EpisodeStore, port: P, response_limit: usize) -> Self {
        Self {
            store,
            port,
            response_limit,
        }
    }
    pub fn root(&self) -> &Path {
        self.store.root()
    }

    /// Legacy executable compatibility. The in-process API is `execute_direct`.
    pub fn execute(&mut self, request: &Request) -> AppResult<String> {
        let result = self.execute_typed(request)?;
        Ok(result.wire_json(request))
    }

    pub fn execute_typed(&mut self, request: &Request) -> AppResult<OperationResult> {
        self.check_profile(request)?;
        self.port.precheck(request)?;
        let operation = Operation::parse(request)?;
        self.execute_operation(request, &operation)
    }

    pub fn execute_direct(&mut self, direct: &DirectRequest) -> AppResult<OperationResult> {
        self.check_profile(direct.request())?;
        self.port.precheck(direct.request())?;
        self.execute_operation(direct.request(), direct.operation())
    }

    fn check_profile(&self, request: &Request) -> AppResult<()> {
        if self.port.request_version() != request.version() {
            Err(AppError::Admission(
                "request and admission profiles differ".into(),
            ))
        } else {
            Ok(())
        }
    }

    fn execute_operation(
        &mut self,
        request: &Request,
        operation: &Operation,
    ) -> AppResult<OperationResult> {
        match operation {
            Operation::Begin {
                authority,
                transition_id,
                unresolved_questions,
            } => self.begin(request, authority, transition_id, unresolved_questions),
            Operation::Transition {
                authority,
                expected_revision,
                transition_id,
                action,
                payload,
            } => self.transition(
                request,
                authority,
                expected_revision,
                transition_id,
                *action,
                payload,
            ),
            Operation::ResumeInitial { authority } => self.resume_initial(request, authority),
            Operation::ValidateResult {
                authority,
                expected_revision,
                result,
            } => self.validate_result(request, authority, expected_revision, result),
            Operation::Recover {
                identity,
                transition,
            } => self.recover(request, identity, transition.as_ref()),
            Operation::Read { identity, revision } => {
                self.read(request, identity, revision.as_ref())
            }
            Operation::History { identity } => self.history(request, identity),
        }
    }

    fn begin(
        &mut self,
        request: &Request,
        authority: &review_episode_core::identity::Authority,
        id: &JsString,
        questions: &[JsValue],
    ) -> AppResult<OperationResult> {
        let key = authority.identity().key().0;
        let observed = self
            .store
            .read(&key)?
            .current
            .map(|s| s.revision().0.clone());
        self.port.admit(request, observed.as_deref())?;
        review_episode_store::test_checkpoint("after_admission");
        let command = BeginCommand::new_utf16(authority.clone(), id.clone(), questions.to_vec())?;
        let context = PossibleCommit {
            root: self.store.root().to_path_buf(),
            operation: "begin",
            identity: authority.identity().clone(),
            transition_id: id.clone(),
            content_digest: Revision(digest(&JsValue::object([
                ("action", JsValue::text("begin")),
                ("unresolvedQuestions", JsValue::Array(questions.to_vec())),
            ]))),
        };
        let outcome = self
            .store
            .write(
                &key,
                observed.as_deref(),
                self.response_limit,
                |current| Ok(write_outcome(reducer::begin(current, &command)?)),
                |state, replay| encode_write_reply(request, state, replay),
            )
            .map_err(|e| map_write_error(e, context.clone()))?;
        Ok(write_result(outcome, context))
    }

    fn transition(
        &mut self,
        request: &Request,
        authority: &review_episode_core::identity::Authority,
        expected: &Revision,
        id: &JsString,
        action: review_episode_core::command::Action,
        payload: &JsValue,
    ) -> AppResult<OperationResult> {
        let key = authority.identity().key().0;
        let observed = self
            .store
            .read(&key)?
            .current
            .map(|s| s.revision().0.clone());
        self.port.admit(request, observed.as_deref())?;
        review_episode_store::test_checkpoint("after_admission");
        let command = TransitionCommand::new_utf16(
            authority.clone(),
            expected.clone(),
            id.clone(),
            action,
            payload.clone(),
        )?;
        let context = PossibleCommit {
            root: self.store.root().to_path_buf(),
            operation: "transition",
            identity: authority.identity().clone(),
            transition_id: id.clone(),
            content_digest: Revision(digest(&command.content())),
        };
        let outcome = self
            .store
            .write(
                &key,
                observed.as_deref(),
                self.response_limit,
                |current| {
                    let current = current.ok_or_else(|| {
                        StoreError::Domain(EpisodeError::revision_conflict(
                            "review episode does not exist",
                        ))
                    })?;
                    Ok(write_outcome(reducer::transition(current, &command)?))
                },
                |state, replay| encode_write_reply(request, state, replay),
            )
            .map_err(|e| map_write_error(e, context.clone()))?;
        Ok(write_result(outcome, context))
    }

    fn resume_initial(
        &mut self,
        request: &Request,
        authority: &review_episode_core::identity::Authority,
    ) -> AppResult<OperationResult> {
        let snapshot = self.store.read(&authority.identity().key().0)?;
        self.port.admit(
            request,
            snapshot.current.as_ref().map(|s| s.revision().0.as_str()),
        )?;
        let current = snapshot
            .current
            .ok_or_else(|| EpisodeError::revision_conflict("review episode does not exist"))?;
        let state = reducer::resume_initial(&current, authority)?;
        self.bounded(
            request,
            OperationResult::ObservedState {
                observed_revision: Some(state.revision().clone()),
                state: Some(state),
            },
        )
    }

    fn validate_result(
        &mut self,
        request: &Request,
        authority: &review_episode_core::identity::Authority,
        expected: &Revision,
        result: &JsValue,
    ) -> AppResult<OperationResult> {
        let snapshot = self.store.read(&authority.identity().key().0)?;
        self.port.admit(
            request,
            snapshot.current.as_ref().map(|s| s.revision().0.as_str()),
        )?;
        let current = snapshot
            .current
            .ok_or_else(|| EpisodeError::revision_conflict("review episode does not exist"))?;
        let validated = reducer::validate_result(&current, authority, expected, result.clone())?;
        self.bounded(
            request,
            OperationResult::ValidatedResult {
                observed_revision: current.revision().clone(),
                result: validated.value,
            },
        )
    }

    fn read_snapshot(&mut self, request: &Request, identity: &Identity) -> AppResult<Snapshot> {
        let snapshot = self.store.read(&identity.key().0)?;
        self.port.admit(
            request,
            snapshot.current.as_ref().map(|s| s.revision().0.as_str()),
        )?;
        Ok(snapshot)
    }

    fn read(
        &mut self,
        request: &Request,
        identity: &Identity,
        revision: Option<&Revision>,
    ) -> AppResult<OperationResult> {
        let snapshot = self.read_snapshot(request, identity)?;
        let observed_revision = snapshot.current.as_ref().map(|s| s.revision().clone());
        let state = match revision {
            None => snapshot.current,
            Some(rev) => snapshot
                .history
                .into_iter()
                .find(|h| h.revision == rev.0)
                .map(|h| h.state),
        };
        self.bounded(
            request,
            OperationResult::ObservedState {
                observed_revision,
                state,
            },
        )
    }

    fn history(&mut self, request: &Request, identity: &Identity) -> AppResult<OperationResult> {
        let snapshot = self.read_snapshot(request, identity)?;
        let observed_revision = snapshot.current.as_ref().map(|s| s.revision().clone());
        let history = snapshot.history.into_iter().map(|h| h.state).collect();
        self.bounded(
            request,
            OperationResult::History {
                observed_revision,
                history,
            },
        )
    }

    fn recover(
        &mut self,
        request: &Request,
        identity: &Identity,
        transition: Option<&(JsString, Revision)>,
    ) -> AppResult<OperationResult> {
        let snapshot = self.read_snapshot(request, identity)?;
        let observed_revision = snapshot.current.as_ref().map(|s| s.revision().clone());
        let Some((id, digest)) = transition else {
            return self.bounded(
                request,
                OperationResult::ObservedState {
                    observed_revision,
                    state: snapshot.current,
                },
            );
        };
        let mut introduced = None;
        for entry in &snapshot.history {
            if let Some(found) = entry.state.handled(id) {
                if found != &digest.value() {
                    return Err(AppError::Domain(EpisodeError::new(
                        "transition identity conflicts with durable content",
                    )));
                }
                introduced = Some(Revision(entry.revision.clone()));
                break;
            }
        }
        self.bounded(
            request,
            OperationResult::Recovery {
                observed_revision,
                committed_revision: introduced,
                state: snapshot.current,
            },
        )
    }

    fn bounded(&self, request: &Request, result: OperationResult) -> AppResult<OperationResult> {
        if result.encoded_len(request) > self.response_limit {
            Err(AppError::ResponseTooLarge)
        } else {
            Ok(result)
        }
    }
}

fn map_write_error(error: StoreError, context: PossibleCommit) -> AppError {
    if matches!(error, StoreError::OutcomeUnknown) {
        AppError::OutcomeUnknown(Box::new(context))
    } else {
        AppError::Store(error)
    }
}

fn write_outcome(outcome: TransitionOutcome) -> WriteDisposition {
    if outcome.is_replay() {
        WriteDisposition::Replay(Box::new(outcome.into_state()))
    } else {
        WriteDisposition::Applied(Box::new(outcome.into_state()))
    }
}

fn write_result(outcome: WriteResult, context: PossibleCommit) -> OperationResult {
    OperationResult::Write {
        status: if outcome.replay {
            WriteStatus::Replay
        } else {
            WriteStatus::Applied
        },
        state: *outcome.state,
        context,
    }
}

fn encode_write_reply(request: &Request, state: &ReviewEpisodeState, replay: bool) -> String {
    // Trusted domain accounting: exactly the selected codec/envelope that the
    // executable would emit. This happens before an applied row is inserted.
    reply(
        request,
        if replay { "replay" } else { "applied" },
        vec![
            ("observedRevision", state.revision().value()),
            ("state", state.value().clone()),
        ],
    )
}
