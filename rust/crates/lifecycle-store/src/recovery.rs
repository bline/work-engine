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

impl SqliteLifecycleStore {
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
        if version != 1 {
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
    if version != 1 {
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
    if effect_mismatch != 0
        || input_mismatch != 0
        || command_mismatch != 0
        || missing_command_journal != 0
        || missing_transition_journal != 0
        || missing_artifact_journal != 0
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
