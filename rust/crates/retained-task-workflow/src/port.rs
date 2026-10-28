use lifecycle_service::{
    CompletedTaskEffect, CompletedTaskEffectAttribution, CompletedTaskEffectReadback, GrantFence,
    QuestionOpenPayload as PortQuestion, ResultPublishPayload as PortResult, TaskCursorBinding,
    TaskInputBinding, TaskOperationCommit as PortCommit, TaskOperationError as PortError,
    TaskOperationPort, ValidatedTaskOperationEntry,
};

use crate::{
    CompletedEffect, CompletedSnapshotReadAttribution, CompletedSnapshotResult,
    CompletedSnapshotUnavailable, OperationCommit, OperationEntry, QuestionOpenPayload,
    ResultPublishPayload, TaskError, TaskStore,
};

impl TaskOperationPort for TaskStore {
    fn lookup_completed_effect(
        &self,
        task_id: &str,
        effect_id: &str,
    ) -> Result<Option<CompletedTaskEffectReadback>, PortError> {
        let effect = TaskStore::lookup_completed_effect(self, effect_id).map_err(port_error)?;
        let Some(effect) = effect else {
            return Ok(None);
        };
        let record = self
            .observe(task_id)
            .map_err(port_error)?
            .ok_or(PortError::Uncertain)?;
        if !record.completed_effects.contains(&effect) {
            return Err(PortError::Invalid);
        }
        let attribution = effect.attribution.map(|a| CompletedTaskEffectAttribution {
            parent_invocation_id: a.parent_invocation_id,
            call_id: a.call_id,
            contract_id: a.contract_id,
            snapshot_id: a.snapshot_id,
            member_id: a.member_id,
            offset: a.offset,
            requested_length: a.requested_length,
            returned_length: a.returned_length,
            result_sha256: a.result_sha256,
        });
        Ok(Some(CompletedTaskEffectReadback {
            task_id: task_id.into(),
            logical_delivery_id: effect.delivery_id,
            effect_id: effect.effect_id,
            receipt_sha256: effect.receipt_digest,
            attribution,
        }))
    }
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
        // Classification may deliberately discard oversized or non-UTF-8 bytes.
        // Check the committed callback bytes before that loss of information.
        if let Some(bytes) = effect.result_bytes.as_deref()
            && (bytes.len() as u64 != effect.returned_length
                || crate::sha256(bytes) != effect.receipt_sha256)
        {
            return Err(PortError::Invalid);
        }
        TaskStore::record_completed_effect(
            self,
            &effect.task_id,
            CompletedEffect {
                effect_id: effect.effect_id.clone(),
                delivery_id: effect.logical_delivery_id.clone(),
                receipt_digest: effect.receipt_sha256.clone(),
                attribution: Some(CompletedSnapshotReadAttribution {
                    schema: "completed-snapshot-read-attribution-v2".into(),
                    parent_invocation_id: effect.parent_invocation_id.clone(),
                    call_id: effect.call_id.clone(),
                    contract_id: effect.contract_id.clone(),
                    snapshot_id: effect.snapshot_id.clone(),
                    member_id: effect.member_id.clone(),
                    offset: effect.offset,
                    requested_length: effect.requested_length,
                    returned_length: effect.returned_length,
                    result_sha256: effect.receipt_sha256.clone(),
                    result: classify_result(effect.result_bytes.as_deref())?,
                }),
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

fn classify_result(bytes: Option<&[u8]>) -> Result<CompletedSnapshotResult, PortError> {
    let Some(bytes) = bytes else {
        return Ok(CompletedSnapshotResult::Unavailable {
            reason: CompletedSnapshotUnavailable::MissingResult,
        });
    };
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Ok(CompletedSnapshotResult::Unavailable {
            reason: CompletedSnapshotUnavailable::NonUtf8,
        });
    };
    if bytes.len() > 4 * 1024 {
        return Ok(CompletedSnapshotResult::Unavailable {
            reason: CompletedSnapshotUnavailable::OverInlineLimit,
        });
    }
    if serde_json::to_vec(text)
        .map_err(|_| PortError::Invalid)?
        .len()
        > 8 * 1024
    {
        return Ok(CompletedSnapshotResult::Unavailable {
            reason: CompletedSnapshotUnavailable::OverEscapedLimit,
        });
    }
    Ok(CompletedSnapshotResult::InlineUtf8 { text: text.into() })
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
