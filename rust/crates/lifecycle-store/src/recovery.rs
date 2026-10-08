use std::fs::{self, File};
use std::path::Path;

use rusqlite::{Connection, OpenFlags};

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
        if version != 2 {
            return Err(StoreError::Unavailable);
        }
        Ok(RecoverySnapshot {
            subjects: count(&inspection, "SELECT COUNT(*) FROM subjects")?,
            commands: count(&inspection, "SELECT COUNT(*) FROM commands")?,
            unresolved_attempts: count(
                &inspection,
                "SELECT COUNT(*) FROM attempts WHERE settlement_kind='unresolved' OR settlement_kind='conflict'",
            )?,
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
    if version != 2 {
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
    if effect_mismatch != 0
        || input_mismatch != 0
        || command_mismatch != 0
        || missing_command_journal != 0
        || missing_transition_journal != 0
        || missing_artifact_journal != 0
        || missing_task_result_journal != 0
        || missing_verifier_journal != 0
    {
        return Err(StoreError::Unavailable);
    }
    Ok(())
}

fn count(connection: &Connection, sql: &str) -> Result<i64, StoreError> {
    connection
        .query_row(sql, [], |row| row.get(0))
        .map_err(|_| StoreError::Unavailable)
}
