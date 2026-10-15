//! Narrow task-owner operations entered through lifecycle native custody.
//!
//! The port owns task records. Lifecycle owns native entry, grant validity and
//! child settlement; a task commit is only established by the owner receipt.

use lifecycle_semantic::SourceReference;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TaskInputBinding {
    pub input_id: String,
    pub task_id: String,
    pub subject_id: String,
    pub cursor_ref: String,
    pub cursor_revision: u64,
    pub owner_revision: u64,
    pub text_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TaskCursorBinding {
    pub task_id: String,
    pub subject_id: String,
    pub cursor_ref: String,
    pub cursor_revision: u64,
    pub owner_revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValidatedTaskOperationEntry {
    pub operation_id: String,
    pub task_id: String,
    pub subject_id: String,
    pub logical_delivery_id: String,
    pub attempt_id: String,
    pub cursor_ref: String,
    pub cursor_revision: u64,
    pub expected_task_revision: u64,
    pub grant_ref: String,
    pub grant_revision: u64,
    pub payload_sha256: String,
    pub contract_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuestionOpenPayload {
    pub question_id: String,
    pub recipient_ref: String,
    pub scope_ref: String,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResultPublishPayload {
    pub result_id: String,
    pub body: String,
    pub source_refs: Vec<SourceReference>,
    pub attribution: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TaskOperationCommit {
    pub operation_id: String,
    pub task_id: String,
    pub owner_revision: u64,
    pub owner_ref: String,
    pub sha256: String,
    pub kind: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompletedTaskEffect {
    pub task_id: String,
    pub logical_delivery_id: String,
    pub effect_id: String,
    pub receipt_sha256: String,
    pub expected_cursor_revision: u64,
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum TaskOperationError {
    #[error("task operation is invalid")]
    Invalid,
    #[error("task operation is stale")]
    Stale,
    #[error("task operation is unauthorized")]
    Unauthorized,
    #[error("task owner is busy")]
    Busy,
    #[error("task operation outcome is uncertain")]
    Uncertain,
}

/// Called by the owner while its short task transaction is held. Lifecycle
/// takes its own immediate grant fence, rechecks current authority, invokes
/// `commit_owner` and releases the lifecycle fence. This preserves the single
/// task-then-lifecycle lock order used by canonical material commits.
pub type GrantFence<'a> = dyn FnMut(&mut dyn FnMut() -> Result<(), TaskOperationError>) -> Result<(), TaskOperationError>
    + 'a;

pub trait TaskOperationPort: Send + Sync {
    /// Compare the exact request text with the already published canonical
    /// input. A direct service-socket caller cannot invent task meaning.
    fn validate_input(
        &self,
        input_id: &str,
        text: &str,
    ) -> Result<TaskInputBinding, TaskOperationError>;

    /// Fresh read-only owner locator for each newly entered child; admission
    /// time revisions cannot stand in for intervening effects or answers.
    fn current_cursor(&self, task_id: &str) -> Result<TaskCursorBinding, TaskOperationError>;

    fn open_question(
        &self,
        entry: &ValidatedTaskOperationEntry,
        payload: QuestionOpenPayload,
        with_grant_fence: &mut GrantFence<'_>,
    ) -> Result<TaskOperationCommit, TaskOperationError>;

    fn publish_result(
        &self,
        entry: &ValidatedTaskOperationEntry,
        payload: ResultPublishPayload,
        with_grant_fence: &mut GrantFence<'_>,
    ) -> Result<TaskOperationCommit, TaskOperationError>;

    fn lookup_operation(
        &self,
        operation_id: &str,
    ) -> Result<Option<TaskOperationCommit>, TaskOperationError>;

    /// Project an already committed lifecycle child consequence into the
    /// canonical task cursor. Exact effect ID makes lost-reply reconciliation
    /// idempotent; this callback itself is not evidence of owner publication.
    fn record_completed_effect(
        &self,
        effect: &CompletedTaskEffect,
    ) -> Result<(), TaskOperationError>;
}
