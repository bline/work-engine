use lifecycle_service::{
    CompletedTaskEffect, GrantFence, QuestionOpenPayload as PortQuestion,
    ResultPublishPayload as PortResult, TaskCursorBinding, TaskInputBinding,
    TaskOperationCommit as PortCommit, TaskOperationError as PortError, TaskOperationPort,
    ValidatedTaskOperationEntry,
};

use crate::{
    CompletedEffect, OperationCommit, OperationEntry, QuestionOpenPayload, ResultPublishPayload,
    TaskError, TaskStore,
};

impl TaskOperationPort for TaskStore {
    fn current_cursor(&self, task_id: &str) -> Result<TaskCursorBinding, PortError> {
        let record = self
            .observe(task_id)
            .map_err(port_error)?
            .ok_or(PortError::Uncertain)?;
        if record.task_id != task_id {
            return Err(PortError::Invalid);
        }
        Ok(TaskCursorBinding {
            task_id: record.task_id.clone(),
            subject_id: record.subject_id,
            cursor_ref: record.task_id,
            cursor_revision: record.cursor_revision,
            owner_revision: record.revision,
        })
    }
    fn record_completed_effect(&self, effect: &CompletedTaskEffect) -> Result<(), PortError> {
        TaskStore::record_completed_effect(
            self,
            &effect.task_id,
            CompletedEffect {
                effect_id: effect.effect_id.clone(),
                delivery_id: effect.logical_delivery_id.clone(),
                receipt_digest: effect.receipt_sha256.clone(),
            },
            effect.expected_cursor_revision,
        )
        .map(|_| ())
        .map_err(port_error)
    }
    fn validate_input(&self, input_id: &str, text: &str) -> Result<TaskInputBinding, PortError> {
        let locator = TaskStore::validate_input(self, input_id, text).map_err(port_error)?;
        Ok(TaskInputBinding {
            input_id: locator.input_id,
            task_id: locator.task_id,
            subject_id: locator.subject_id,
            cursor_ref: locator.cursor_ref,
            cursor_revision: locator.cursor_revision,
            owner_revision: locator.owner_revision,
            text_sha256: crate::sha256(text.as_bytes()),
        })
    }
    fn open_question(
        &self,
        entry: &ValidatedTaskOperationEntry,
        payload: PortQuestion,
        with_grant_fence: &mut GrantFence<'_>,
    ) -> Result<PortCommit, PortError> {
        if entry.contract_id != "task.question.open.v1" || entry.cursor_ref != entry.task_id {
            return Err(PortError::Invalid);
        }
        let entry = local_entry(entry);
        let payload = QuestionOpenPayload {
            question_id: payload.question_id,
            recipient_ref: payload.recipient_ref,
            scope_ref: payload.scope_ref,
            text: payload.text,
        };
        let mut fence = |commit: &mut dyn FnMut() -> Result<(), TaskError>| {
            with_grant_fence(&mut || commit().map_err(port_error)).map_err(task_error)
        };
        TaskStore::open_question(self, &entry, &payload, &mut fence)
            .map(port_commit)
            .map_err(port_error)
    }

    fn publish_result(
        &self,
        entry: &ValidatedTaskOperationEntry,
        payload: PortResult,
        with_grant_fence: &mut GrantFence<'_>,
    ) -> Result<PortCommit, PortError> {
        if entry.contract_id != "task.result.publish.v1" || entry.cursor_ref != entry.task_id {
            return Err(PortError::Invalid);
        }
        let entry = local_entry(entry);
        let payload = ResultPublishPayload {
            result_id: payload.result_id,
            body: payload.body,
            source_refs: payload.source_refs,
            attribution: payload.attribution,
        };
        let mut fence = |commit: &mut dyn FnMut() -> Result<(), TaskError>| {
            with_grant_fence(&mut || commit().map_err(port_error)).map_err(task_error)
        };
        TaskStore::publish_result(self, &entry, &payload, &mut fence)
            .map(port_commit)
            .map_err(port_error)
    }

    fn lookup_operation(&self, operation_id: &str) -> Result<Option<PortCommit>, PortError> {
        TaskStore::lookup_operation(self, operation_id)
            .map(|value| value.map(port_commit))
            .map_err(port_error)
    }
}

fn local_entry(value: &ValidatedTaskOperationEntry) -> OperationEntry {
    OperationEntry {
        operation_id: value.operation_id.clone(),
        task_id: value.task_id.clone(),
        subject_id: value.subject_id.clone(),
        delivery_id: value.logical_delivery_id.clone(),
        attempt_id: value.attempt_id.clone(),
        expected_task_revision: value.expected_task_revision,
        expected_cursor_revision: value.cursor_revision,
        grant_ref: value.grant_ref.clone(),
        grant_revision: value.grant_revision,
        payload_sha256: value.payload_sha256.clone(),
    }
}

fn port_commit(value: OperationCommit) -> PortCommit {
    PortCommit {
        operation_id: value.operation_id,
        task_id: value.task_id.clone(),
        owner_revision: value.owner_revision,
        owner_ref: value.task_id,
        sha256: value.digest,
        kind: value.kind,
    }
}

fn port_error(value: TaskError) -> PortError {
    match value {
        TaskError::Stale | TaskError::PendingQuestion => PortError::Stale,
        TaskError::Unauthorized => PortError::Unauthorized,
        TaskError::Busy => PortError::Busy,
        TaskError::Unavailable | TaskError::Uncertain => PortError::Uncertain,
        TaskError::Storage(rusqlite::Error::SqliteFailure(ref failure, _))
            if matches!(
                failure.code,
                rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
            ) =>
        {
            PortError::Busy
        }
        TaskError::Storage(_) | TaskError::Encoding(_) => PortError::Uncertain,
        TaskError::Invalid | TaskError::Conflict => PortError::Invalid,
    }
}

fn task_error(value: PortError) -> TaskError {
    match value {
        PortError::Invalid => TaskError::Invalid,
        PortError::Unauthorized => TaskError::Unauthorized,
        PortError::Stale => TaskError::Stale,
        PortError::Busy => TaskError::Busy,
        PortError::Uncertain => TaskError::Uncertain,
    }
}
