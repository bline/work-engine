use std::fs::{self, File};
use std::path::Path;

use rusqlite::{Connection, OpenFlags, OptionalExtension};

use crate::{SqliteLifecycleStore, StoreError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecoverySnapshot {
    pub subjects: i64,
    pub commands: i64,
    pub unresolved_attempts: i64,
    pub journal_cursor: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryDisposition {
    PreparedNeedsGrant,
    EnteredUncertain,
    CompletedSettled,
    BlockedOrConflicted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryEntry {
    pub input_id: String,
    pub effect_id: String,
    pub attempt_id: Option<String>,
    pub incarnation_id: Option<String>,
    pub disposition: RecoveryDisposition,
}

type RecoveryRow = (
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

impl SqliteLifecycleStore {
    /// Classify exact persisted work without inferring that an entered attempt is safe to replay.
    pub fn load_recovery_entries(&self) -> Result<Vec<RecoveryEntry>, StoreError> {
        self.verify_identity()?;
        check_coherence(&self.connection)?;
        let mut statement=self.connection.prepare(
            "SELECT i.input_id,e.effect_id,e.state,a.attempt_id,a.incarnation_id,a.outcome,a.settlement_kind FROM effects e JOIN inputs i ON i.input_id=e.input_id LEFT JOIN attempts a ON a.effect_id=e.effect_id WHERE e.state!='cancelled' ORDER BY i.subject_id,i.sequence",
        ).map_err(|_|StoreError::Unavailable)?;
        let rows = statement
            .query_map([], |row| -> rusqlite::Result<RecoveryRow> {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            })
            .map_err(|_| StoreError::Unavailable)?;
        let mut entries = Vec::new();
        for row in rows {
            let (input_id, effect_id, state, attempt_id, incarnation_id, outcome, settlement) =
                row.map_err(|_| StoreError::Unavailable)?;
            let disposition = match (state.as_str(), outcome.as_deref(), settlement.as_deref()) {
                ("prepared", None, None) => RecoveryDisposition::PreparedNeedsGrant,
                (_, Some("completed"), Some("established")) => {
                    RecoveryDisposition::CompletedSettled
                }
                ("entered", _, _) => RecoveryDisposition::EnteredUncertain,
                _ => RecoveryDisposition::BlockedOrConflicted,
            };
            entries.push(RecoveryEntry {
                input_id,
                effect_id,
                attempt_id,
                incarnation_id,
                disposition,
            });
        }
        let mut sessions = self.connection.prepare(
            "SELECT session_id,incarnation_id,allocation_state FROM native_sessions ORDER BY created_wall_ms,session_id"
        ).map_err(|_| StoreError::Unavailable)?;
        let sessions = sessions
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|_| StoreError::Unavailable)?;
        for session in sessions {
            let (id, incarnation, state) = session.map_err(|_| StoreError::Unavailable)?;
            entries.push(RecoveryEntry {
                input_id: format!("native_session:{id}"),
                effect_id: id,
                attempt_id: None,
                incarnation_id: Some(incarnation),
                disposition: match state.as_str() {
                    "intent" => RecoveryDisposition::EnteredUncertain,
                    "bound" => RecoveryDisposition::CompletedSettled,
                    _ => RecoveryDisposition::BlockedOrConflicted,
                },
            });
        }
        let mut turns = self.connection.prepare(
            "SELECT invocation_id,attempt_id,s.incarnation_id,t.state FROM native_turns t JOIN native_sessions s ON s.session_id=t.session_id ORDER BY t.entered_wall_ms,t.invocation_id"
        ).map_err(|_| StoreError::Unavailable)?;
        let turns = turns
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .map_err(|_| StoreError::Unavailable)?;
        for turn in turns {
            let (id, attempt, incarnation, state) = turn.map_err(|_| StoreError::Unavailable)?;
            entries.push(RecoveryEntry {
                input_id: format!("native_turn:{id}"),
                effect_id: id,
                attempt_id: Some(attempt),
                incarnation_id: Some(incarnation),
                disposition: match state.as_str() {
                    "completed_settled" => RecoveryDisposition::CompletedSettled,
                    "entered_uncertain" => RecoveryDisposition::EnteredUncertain,
                    _ => RecoveryDisposition::BlockedOrConflicted,
                },
            });
        }
        Ok(entries)
    }
    /// Validate one coherent database view before exposing new admission.
    pub fn load_recovery(&self) -> Result<RecoverySnapshot, StoreError> {
        self.verify_identity()?;
        check_coherence(&self.connection)?;
        let subjects = count(&self.connection, "SELECT COUNT(*) FROM subjects")?;
        let commands = count(&self.connection, "SELECT COUNT(*) FROM commands")?;
        let unresolved_attempts = count(
            &self.connection,
            "SELECT COUNT(*) FROM attempts WHERE settlement_kind='unresolved' OR settlement_kind='conflict'",
        )? + count(
            &self.connection,
            "SELECT COUNT(*) FROM native_sessions WHERE allocation_state!='bound'",
        )? + count(
            &self.connection,
            "SELECT COUNT(*) FROM native_turns WHERE state!='completed_settled'",
        )? + count(
            &self.connection,
            "SELECT COUNT(*) FROM native_operations WHERE state!='completed_settled'",
        )?;
        let journal_cursor = count(
            &self.connection,
            "SELECT COALESCE(MAX(sequence),0) FROM journal",
        )?;
        Ok(RecoverySnapshot {
            subjects,
            commands,
            unresolved_attempts,
            journal_cursor,
        })
    }

    /// SQLite produces a coherent image including WAL state; no main-file-only copy is used.
    pub fn snapshot_to(&self, destination: &Path) -> Result<RecoverySnapshot, StoreError> {
        self.verify_identity()?;
        if destination.exists() {
            return Err(StoreError::ClaimConflict);
        }
        let path = destination.to_str().ok_or(StoreError::Rejected)?;
        self.connection
            .execute("VACUUM INTO ?1", [path])
            .map_err(|_| StoreError::Unavailable)?;
        File::open(destination)
            .and_then(|file| file.sync_all())
            .map_err(|_| StoreError::Unavailable)?;
        let parent = destination.parent().ok_or(StoreError::Rejected)?;
        File::open(parent)
            .and_then(|file| file.sync_all())
            .map_err(|_| StoreError::Unavailable)?;
        let inspection = Connection::open_with_flags(destination, OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| StoreError::Unavailable)?;
        check_coherence(&inspection)?;
        let version: i64 = inspection
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|_| StoreError::Unavailable)?;
        if !matches!(version, 2..=7) {
            return Err(StoreError::Unavailable);
        }
        Ok(RecoverySnapshot {
            subjects: count(&inspection, "SELECT COUNT(*) FROM subjects")?,
            commands: count(&inspection, "SELECT COUNT(*) FROM commands")?,
            unresolved_attempts: count(
                &inspection,
                "SELECT COUNT(*) FROM attempts WHERE settlement_kind='unresolved' OR settlement_kind='conflict'",
            )? + if version >= 3 {
                count(
                    &inspection,
                    "SELECT COUNT(*) FROM native_sessions WHERE allocation_state!='bound'",
                )? + count(
                    &inspection,
                    "SELECT COUNT(*) FROM native_turns WHERE state!='completed_settled'",
                )? + count(
                    &inspection,
                    "SELECT COUNT(*) FROM native_operations WHERE state!='completed_settled'",
                )?
            } else {
                0
            },
            journal_cursor: count(&inspection, "SELECT COALESCE(MAX(sequence),0) FROM journal")?,
        })
    }

    /// Orphan staging blobs are complete or partial unreferenced data, never evidence.
    pub fn collect_orphan_staging(&self) -> Result<usize, StoreError> {
        self.verify_identity()?;
        let staging = self.root.join("artifacts/staging");
        if !staging.exists() {
            return Ok(0);
        }
        let mut removed = 0;
        for entry in fs::read_dir(&staging).map_err(|_| StoreError::Unavailable)? {
            let entry = entry.map_err(|_| StoreError::Unavailable)?;
            if entry
                .file_type()
                .map_err(|_| StoreError::Unavailable)?
                .is_file()
            {
                fs::remove_file(entry.path()).map_err(|_| StoreError::Unavailable)?;
                removed += 1;
            }
        }
        File::open(&staging)
            .and_then(|file| file.sync_all())
            .map_err(|_| StoreError::Unavailable)?;
        Ok(removed)
    }
}

pub(super) fn check_coherence(connection: &Connection) -> Result<(), StoreError> {
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|_| StoreError::Unavailable)?;
    if !matches!(version, 2..=7) {
        return Err(StoreError::Unavailable);
    }
    let foreign_key_error: bool = connection
        .prepare("PRAGMA foreign_key_check")
        .map_err(|_| StoreError::Unavailable)?
        .exists([])
        .map_err(|_| StoreError::Unavailable)?;
    if foreign_key_error {
        return Err(StoreError::Unavailable);
    }
    if version >= 4 {
        let revision: i64 = connection
            .query_row(
                "SELECT revision FROM admin_custody_revision WHERE singleton=1",
                [],
                |row| row.get(0),
            )
            .map_err(|_| StoreError::Unavailable)?;
        // Every mutable owner table has an insert/update/delete trigger. An
        // incomplete clock cannot qualify a capsule as the current source.
        let trigger_count = count(
            connection,
            "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name GLOB 'custody_revision_*'",
        )?;
        let expected = match version {
            7 => 126,
            6 => 123,
            5 => 111,
            _ => 102,
        };
        if revision < 0 || trigger_count != expected {
            return Err(StoreError::Unavailable);
        }
        if version >= 6 {
            crate::admin::check_held_coherence(connection)?;
        }
        if version == 6 {
            let index: Option<String>=connection.query_row(
                "SELECT sql FROM sqlite_master WHERE type='index' AND name='one_native_generation_session'",
                [],|r|r.get(0)).optional().map_err(|_| StoreError::Unavailable)?;
            if index.as_ref().is_none_or(|sql| {
                let compact = sql
                    .to_ascii_lowercase()
                    .split_whitespace()
                    .collect::<String>();
                !compact.contains("onnative_sessions(subject_id,context_id)")
                    || !compact.contains("whererolein('domain','successor')")
            }) {
                return Err(StoreError::Unavailable);
            }
        }
        if version == 7 {
            check_restart_epoch_coherence(connection)?;
        }
    }
    let effect_mismatch = count(
        connection,
        "SELECT COUNT(*) FROM effects e LEFT JOIN attempts a ON a.effect_id=e.effect_id WHERE (e.state='prepared' AND a.attempt_id IS NOT NULL) OR (e.state IN ('entered','applied') AND a.attempt_id IS NULL)",
    )?;
    let input_mismatch = count(
        connection,
        "SELECT COUNT(*) FROM inputs i LEFT JOIN effects e ON e.input_id=i.input_id AND e.state!='cancelled' WHERE (i.state='queued' AND e.effect_id IS NOT NULL) OR (i.state IN ('prepared','entered','succeeded','failed_blocked','unknown') AND e.effect_id IS NULL)",
    )?;
    let command_mismatch = count(
        connection,
        "SELECT COUNT(*) FROM commands c LEFT JOIN inputs i ON i.delivery_id=c.outcome_ref WHERE c.outcome_kind='enqueued' AND i.input_id IS NULL",
    )?;
    let missing_command_journal = count(
        connection,
        "SELECT COUNT(*) FROM commands c WHERE NOT EXISTS (SELECT 1 FROM journal j WHERE j.subject_id=c.subject_id AND j.event_kind=c.outcome_kind AND j.event_ref=c.command_id)",
    )?;
    let missing_transition_journal = count(
        connection,
        "SELECT COUNT(*) FROM transition_evidence e JOIN transitions t ON t.transition_id=e.transition_id WHERE NOT EXISTS (SELECT 1 FROM journal j WHERE j.subject_id=t.subject_id AND j.event_kind='transition_fact' AND j.event_ref=e.source_id)",
    )?;
    let missing_artifact_journal = count(
        connection,
        "SELECT COUNT(*) FROM artifacts a WHERE NOT EXISTS (SELECT 1 FROM journal j WHERE j.subject_id=a.subject_id AND j.event_kind='artifact_published' AND j.event_ref=a.digest_hex)",
    )?;
    let missing_task_result_journal = count(
        connection,
        "SELECT COUNT(*) FROM task_results r JOIN attempts a ON a.attempt_id=r.attempt_id JOIN effects e ON e.effect_id=a.effect_id WHERE NOT EXISTS (SELECT 1 FROM journal j WHERE j.subject_id=e.subject_id AND j.event_kind='task_result_owned' AND j.event_ref=r.attempt_id)",
    )?;
    let missing_verifier_journal = count(
        connection,
        "SELECT COUNT(*) FROM verifier_results v JOIN transitions t ON t.transition_id=v.transition_id WHERE NOT EXISTS (SELECT 1 FROM journal j WHERE j.subject_id=t.subject_id AND j.event_kind='verifier_resolved' AND j.event_ref=v.transition_id)",
    )?;
    let native_mismatch = if version >= 3 {
        let transition_mismatch = if version >= 5 {
            count(
                connection,
                "SELECT COUNT(*) FROM transitions t JOIN native_turns n ON n.invocation_id=t.native_basis_source \
                 JOIN native_sessions s ON s.session_id=n.session_id WHERE s.subject_id!=t.subject_id OR \
                 s.context_id!=t.predecessor_context OR n.purpose!='domain_work' OR \
                 (n.state!='completed_settled' AND NOT (t.stage='quiescing' AND n.state='entered_uncertain' \
                  AND EXISTS(SELECT 1 FROM native_delivery_segments d WHERE d.invocation_id=n.invocation_id \
                             AND d.state='entered')))",
            )?
        } else {
            count(
                connection,
                "SELECT COUNT(*) FROM transitions t JOIN native_turns n ON n.invocation_id=t.native_basis_source \
                 JOIN native_sessions s ON s.session_id=n.session_id WHERE s.subject_id!=t.subject_id OR \
                 s.context_id!=t.predecessor_context OR n.state!='completed_settled' OR n.purpose!='domain_work'",
            )?
        };
        transition_mismatch
            + count(
                connection,
                "SELECT COUNT(*) FROM native_verifier_results v JOIN transitions t ON t.transition_id=v.transition_id \
                 WHERE v.source_id!='native:'||t.native_basis_source OR NOT EXISTS \
                 (SELECT 1 FROM journal j WHERE j.subject_id=t.subject_id AND j.event_kind='verifier_resolved' AND j.event_ref=v.transition_id)",
            )?
            + count(
                connection,
                "SELECT COUNT(*) FROM native_turns WHERE state='completed_settled' AND \
                 (final_text IS NULL OR final_sha256 IS NULL OR turn_id IS NULL OR history_transport_session IS NULL OR history_sequence IS NULL)",
            )?
    } else {
        0
    };
    if effect_mismatch != 0
        || input_mismatch != 0
        || command_mismatch != 0
        || missing_command_journal != 0
        || missing_transition_journal != 0
        || missing_artifact_journal != 0
        || missing_task_result_journal != 0
        || missing_verifier_journal != 0
        || native_mismatch != 0
    {
        return Err(StoreError::Unavailable);
    }
    Ok(())
}

fn check_restart_epoch_coherence(connection: &Connection) -> Result<(), StoreError> {
    // The two partial indexes are part of the single-current-session contract.
    // Count and exact names alone cannot detect a replaced no-op index/trigger.
    for (name, state) in [
        ("one_active_native_execution_epoch", "active"),
        ("one_authorized_native_execution_epoch", "authorized"),
    ] {
        let definition: Option<(String, String)> = connection
            .query_row(
                "SELECT tbl_name,sql FROM sqlite_master WHERE type='index' AND name=?1",
                [name],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        if definition.as_ref().is_none_or(|(table, sql)| {
            let compact = sql
                .to_ascii_lowercase()
                .split_whitespace()
                .collect::<String>();
            table != "native_execution_epochs"
                || compact
                    != format!("createuniqueindex{name}onnative_execution_epochs(subject_id,context_id)wherestate='{state}'")
        }) {
            return Err(StoreError::Unavailable);
        }
    }
    for (name, event) in [
        (
            "custody_revision_native_execution_epochs_insert",
            "afterinsert",
        ),
        (
            "custody_revision_native_execution_epochs_update",
            "afterupdate",
        ),
        (
            "custody_revision_native_execution_epochs_delete",
            "afterdelete",
        ),
    ] {
        let definition: Option<(String, String)> = connection
            .query_row(
                "SELECT tbl_name,sql FROM sqlite_master WHERE type='trigger' AND name=?1",
                [name],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(|_| StoreError::Unavailable)?;
        if definition.as_ref().is_none_or(|(table, sql)| {
            let compact = sql
                .to_ascii_lowercase()
                .split_whitespace()
                .collect::<String>();
            table != "native_execution_epochs"
                || compact
                    != format!("createtrigger{name}{event}onnative_execution_epochsbeginupdateadmin_custody_revisionsetrevision=revision+1wheresingleton=1;end")
        }) {
            return Err(StoreError::Unavailable);
        }
    }
    let mismatch = count(
        connection,
        "SELECT COUNT(*) FROM native_sessions s WHERE s.role IN ('domain','successor') AND \
         (SELECT COUNT(*) FROM native_execution_epochs e WHERE e.target_session_id=s.session_id \
          AND e.subject_id=s.subject_id AND e.context_id=s.context_id AND e.state IN ('active','historical'))!=1",
    )? + count(
        connection,
        "SELECT COUNT(*) FROM native_execution_epochs e WHERE \
         ((e.state IN ('active','historical') AND NOT EXISTS(SELECT 1 FROM native_sessions s \
             WHERE s.session_id=e.target_session_id AND s.subject_id=e.subject_id \
               AND s.context_id=e.context_id AND s.role IN ('domain','successor'))) \
          OR (e.state='authorized' AND EXISTS(SELECT 1 FROM native_sessions s WHERE s.session_id=e.target_session_id)) \
          OR (e.epoch>0 AND NOT EXISTS(SELECT 1 FROM native_execution_epochs p \
             WHERE p.subject_id=e.subject_id AND p.context_id=e.context_id AND p.epoch=e.epoch-1 \
               AND p.target_session_id=e.predecessor_session_id \
               AND ((e.state='authorized' AND p.state='active') \
                 OR (e.state='active' AND p.state='historical') \
                 OR (e.state='historical' AND p.state='historical')))) \
          OR (e.epoch=0 AND e.state='historical' AND NOT EXISTS(SELECT 1 FROM native_execution_epochs n \
             WHERE n.subject_id=e.subject_id AND n.context_id=e.context_id AND n.epoch=1)))",
    )? + count(
        connection,
        "SELECT COUNT(*) FROM (SELECT subject_id,context_id, \
           SUM(state='active') active_count, SUM(state='authorized') authorized_count, \
           SUM(state='historical') historical_count FROM native_execution_epochs \
           GROUP BY subject_id,context_id) x WHERE active_count!=1 OR authorized_count>1",
    )? + count(
        connection,
        "SELECT COUNT(*) FROM native_execution_epochs e WHERE e.epoch>0 AND \
         (length(e.intent_sha256)!=64 OR length(e.selection_sha256)!=64 \
          OR length(e.profile_digest)!=64 OR length(e.binary_sha256)!=64 \
          OR length(e.protocol_digest)!=64 OR length(e.snapshot_manifest_sha256)!=64 \
          OR e.intent_sha256 GLOB '*[^0-9A-Fa-f]*' \
          OR e.selection_sha256 GLOB '*[^0-9A-Fa-f]*' \
          OR e.profile_digest GLOB '*[^0-9A-Fa-f]*' \
          OR e.binary_sha256 GLOB '*[^0-9A-Fa-f]*' \
          OR e.protocol_digest GLOB '*[^0-9A-Fa-f]*' \
          OR e.snapshot_manifest_sha256 GLOB '*[^0-9A-Fa-f]*')",
    )?;
    if mismatch != 0 {
        return Err(StoreError::Unavailable);
    }
    Ok(())
}

fn count(connection: &Connection, sql: &str) -> Result<i64, StoreError> {
    connection
        .query_row(sql, [], |row| row.get(0))
        .map_err(|_| StoreError::Unavailable)
}
