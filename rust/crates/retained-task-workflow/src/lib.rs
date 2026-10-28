//! Canonical, durable meaning for a retained repository-analysis task.
//! Lifecycle owns delivery, native execution, tool effects, and custody.

use std::{
    fs::{self, File, OpenOptions},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
    sync::Mutex,
    time::Duration,
};
#[cfg(feature = "controlled-proof")]
use std::{path::PathBuf, time::Instant};

use lifecycle_semantic::{
    ObligationKind, RequiredObligation, SourceAvailability, SourceKind, SourceMaterial,
    SourceReference, SourceTrust,
};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use thiserror::Error;

mod material;
mod port;

pub const PROFILE: &str = "retained-repository-analysis-v1";
pub const OWNER_NAMESPACE: &str = "retained-task-workflow";
const MAX_TEXT: usize = 8 * 1024;
// These are the experimental inline limits for this retained S10 profile.
const EFFECT_INLINE_BYTES: usize = 4 * 1024;
const EFFECT_ESCAPED_BYTES: usize = 8 * 1024;
type TaskGrantFence<'a> =
    dyn FnMut(&mut dyn FnMut() -> Result<(), TaskError>) -> Result<(), TaskError> + 'a;
type AdmissionBuilder<'a> = dyn FnMut(&str) -> Result<AdmissionCommand, TaskError> + 'a;
type CommandRow = (String, String, String, Vec<u8>);
type CompletedEffectRow = (
    String,
    String,
    String,
    Option<i64>,
    Option<i64>,
    Option<String>,
);

// Keep the existing six tables. Schema 2 retains each completed effect's
// original owner receipt alongside its identity.
const OWNER_SCHEMA_SQL: &str = "CREATE TABLE IF NOT EXISTS tasks (
                 task_id TEXT PRIMARY KEY, subject_id TEXT NOT NULL UNIQUE,
                 record_json BLOB NOT NULL
               );
               CREATE TABLE IF NOT EXISTS commands (
                 command_id TEXT PRIMARY KEY, kind TEXT NOT NULL,
                 payload_digest TEXT NOT NULL, task_id TEXT NOT NULL,
                 commit_json BLOB NOT NULL
               );
               CREATE TABLE IF NOT EXISTS operations (
                 operation_id TEXT PRIMARY KEY, kind TEXT NOT NULL,
                 payload_digest TEXT NOT NULL, entry_digest TEXT NOT NULL,
                 commit_json BLOB NOT NULL
               );
               CREATE TABLE IF NOT EXISTS completed_effects (
                 effect_id TEXT PRIMARY KEY, task_id TEXT NOT NULL,
                 delivery_id TEXT NOT NULL, receipt_digest TEXT NOT NULL, owner_revision INTEGER,
                 cursor_revision INTEGER, record_digest TEXT
               );
               CREATE TABLE IF NOT EXISTS admission_commands (
                 command_id TEXT PRIMARY KEY, request_digest TEXT NOT NULL,
                 command_json TEXT NOT NULL
               );
               CREATE TABLE IF NOT EXISTS control_commands (
                 command_id TEXT PRIMARY KEY, kind TEXT NOT NULL,
                 request_sha256 TEXT NOT NULL, service_command_json TEXT NOT NULL
               );
               CREATE TABLE IF NOT EXISTS store_identity (
                 singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                 store_id TEXT NOT NULL UNIQUE,
                 schema_version INTEGER NOT NULL CHECK(schema_version = 2)
               );";

// The stopped v1 migration and recognized version-0 bootstrap each validate
// this historical layout directly; new schema edits cannot redefine it.
const OWNER_SCHEMA_V1_SQL: &str = "CREATE TABLE IF NOT EXISTS tasks (
                 task_id TEXT PRIMARY KEY, subject_id TEXT NOT NULL UNIQUE,
                 record_json BLOB NOT NULL
               );
               CREATE TABLE IF NOT EXISTS commands (
                 command_id TEXT PRIMARY KEY, kind TEXT NOT NULL,
                 payload_digest TEXT NOT NULL, task_id TEXT NOT NULL,
                 commit_json BLOB NOT NULL
               );
               CREATE TABLE IF NOT EXISTS operations (
                 operation_id TEXT PRIMARY KEY, kind TEXT NOT NULL,
                 payload_digest TEXT NOT NULL, entry_digest TEXT NOT NULL,
                 commit_json BLOB NOT NULL
               );
               CREATE TABLE IF NOT EXISTS completed_effects (
                 effect_id TEXT PRIMARY KEY, task_id TEXT NOT NULL,
                 delivery_id TEXT NOT NULL, receipt_digest TEXT NOT NULL
               );
               CREATE TABLE IF NOT EXISTS admission_commands (
                 command_id TEXT PRIMARY KEY, request_digest TEXT NOT NULL,
                 command_json TEXT NOT NULL
               );
               CREATE TABLE IF NOT EXISTS control_commands (
                 command_id TEXT PRIMARY KEY, kind TEXT NOT NULL,
                 request_sha256 TEXT NOT NULL, service_command_json TEXT NOT NULL
               );
               CREATE TABLE IF NOT EXISTS store_identity (
                 singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
                 store_id TEXT NOT NULL UNIQUE,
                 schema_version INTEGER NOT NULL CHECK(schema_version = 1)
               );";

fn rebuild_completed_effects(connection: &Connection, ddl: &str) -> Result<(), TaskError> {
    connection.execute_batch("ALTER TABLE completed_effects RENAME TO completed_effects_v1;")?;
    connection.execute_batch(ddl)?;
    connection.execute_batch(
        "INSERT INTO completed_effects(effect_id,task_id,delivery_id,receipt_digest)
        SELECT effect_id,task_id,delivery_id,receipt_digest FROM completed_effects_v1;
        DROP TABLE completed_effects_v1;",
    )?;
    Ok(())
}

type SchemaObject = (String, String, String, Option<String>);

fn schema_objects(connection: &Connection) -> Result<Vec<SchemaObject>, TaskError> {
    let mut statement = connection.prepare(
        "SELECT type, name, tbl_name, sql FROM sqlite_master ORDER BY type, name, tbl_name",
    )?;
    let rows = statement.query_map([], |row| {
        let sql: Option<String> = row.get(3)?;
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            sql.map(|definition| {
                // SQLite preserves formatting from the creating binary. The
                // old owner used the same DDL with different indentation.
                definition.split_whitespace().collect::<Vec<_>>().join(" ")
            }),
        ))
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(TaskError::from)
}

fn validate_owner_schema(connection: &Connection, legacy: bool) -> Result<(), TaskError> {
    let expected = Connection::open_in_memory()?;
    if legacy {
        expected.execute_batch(OWNER_SCHEMA_V1_SQL)?;
    } else {
        expected.execute_batch(OWNER_SCHEMA_SQL)?;
    }
    let mut expected_objects = schema_objects(&expected)?;
    if legacy {
        expected_objects.retain(|(_, _, table, _)| table != "store_identity");
    }
    if schema_objects(connection)? != expected_objects {
        return Err(TaskError::Invalid);
    }
    Ok(())
}

/// Explicit stopped-store migration for a disposable v1 owner store. Host
/// startup never invokes this function.
pub fn migrate_v1_to_v2(path: &Path) -> Result<(), TaskError> {
    migrate_v1_to_v2_inner(path, false)
}

fn migrate_v1_to_v2_inner(path: &Path, fail_after_ddl: bool) -> Result<(), TaskError> {
    if !path.is_absolute() {
        return Err(TaskError::Invalid);
    }
    let lock_path = path.with_extension("owner.lock");
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(lock_path)
        .map_err(|_| TaskError::Unavailable)?;
    lock.try_lock().map_err(|_| TaskError::Busy)?;
    let mut connection = Connection::open(path)?;
    connection.busy_timeout(Duration::from_secs(2))?;
    let version: i64 = connection.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version != 1 {
        return Err(TaskError::Invalid);
    }
    let old_expected = Connection::open_in_memory()?;
    old_expected.execute_batch(OWNER_SCHEMA_V1_SQL)?;
    if schema_objects(&connection)? != schema_objects(&old_expected)? {
        return Err(TaskError::Invalid);
    }
    let (store_id, identity_version): (String, i64) = connection.query_row(
        "SELECT store_id,schema_version FROM store_identity WHERE singleton=1",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    if identity_version != 1
        || store_id.len() != 32
        || !store_id
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(TaskError::Invalid);
    }
    let v2_expected = Connection::open_in_memory()?;
    v2_expected.execute_batch(OWNER_SCHEMA_SQL)?;
    let identity_ddl: String = v2_expected.query_row(
        "SELECT sql FROM sqlite_master WHERE name='store_identity'",
        [],
        |r| r.get(0),
    )?;
    let effect_ddl: String = v2_expected.query_row(
        "SELECT sql FROM sqlite_master WHERE name='completed_effects'",
        [],
        |r| r.get(0),
    )?;
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    rebuild_completed_effects(&tx, &effect_ddl)?;
    tx.execute_batch("DROP TABLE store_identity;")?;
    tx.execute_batch(&identity_ddl)?;
    tx.execute(
        "INSERT INTO store_identity(singleton,store_id,schema_version) VALUES(1,?1,2)",
        [&store_id],
    )?;
    tx.execute_batch("PRAGMA user_version=2;")?;
    if fail_after_ddl {
        return Err(TaskError::Invalid);
    }
    validate_owner_schema(&tx, false)?;
    tx.commit()?;
    Ok(())
}

#[derive(Debug, Error)]
pub enum TaskError {
    #[error("task input is invalid")]
    Invalid,
    #[error("task command conflicts with existing identity")]
    Conflict,
    #[error("task or operation is unavailable")]
    Unavailable,
    #[error("task authority or revision is stale")]
    Stale,
    #[error("task authority is invalid")]
    Unauthorized,
    #[error("task operation is blocked by an unresolved question")]
    PendingQuestion,
    #[error("task owner is busy")]
    Busy,
    #[error("task owner outcome is uncertain")]
    Uncertain,
    #[error("task storage failed: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("task encoding failed: {0}")]
    Encoding(#[from] serde_json::Error),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubjectKind {
    RepositoryAnalysis,
    DesignInquiry,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactKind {
    Governing,
    CanonicalRecord,
    Decision,
    Commitment,
    Uncertainty,
    HumanInteraction,
    CanonicalReference,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactInput {
    pub id: String,
    pub kind: FactKind,
    pub text: String,
    #[serde(default = "default_current")]
    pub current: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubmitTask {
    pub command_id: String,
    pub task_id: String,
    pub subject_id: String,
    pub kind: SubjectKind,
    pub objective: String,
    pub engineering_question: String,
    pub snapshot_ref: String,
    #[serde(default)]
    pub facts: Vec<FactInput>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnswerQuestion {
    pub command_id: String,
    pub task_id: String,
    pub question_id: String,
    pub recipient_ref: String,
    pub scope_ref: String,
    pub answer: String,
    pub expected_revision: u64,
    pub authority_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionState {
    Pending,
    Answered {
        answer: String,
        authority_ref: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Question {
    pub id: String,
    pub recipient_ref: String,
    pub scope_ref: String,
    pub text: String,
    pub state: QuestionState,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompletedEffect {
    pub effect_id: String,
    pub delivery_id: String,
    pub receipt_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<CompletedSnapshotReadAttribution>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedSnapshotReadAttribution {
    pub schema: String,
    pub parent_invocation_id: String,
    pub call_id: String,
    pub contract_id: String,
    pub snapshot_id: String,
    pub member_id: String,
    pub offset: u64,
    pub requested_length: u64,
    pub returned_length: u64,
    pub result_sha256: String,
    pub result: CompletedSnapshotResult,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "availability", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompletedSnapshotResult {
    InlineUtf8 {
        text: String,
    },
    Unavailable {
        reason: CompletedSnapshotUnavailable,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletedSnapshotUnavailable {
    MissingResult,
    NonUtf8,
    OverInlineLimit,
    OverEscapedLimit,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AttributedResult {
    pub result_id: String,
    pub operation_id: String,
    pub body: String,
    pub source_refs: Vec<SourceReference>,
    pub attribution: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TaskInput {
    pub command_id: String,
    pub input_id: String,
    pub engineering_question: String,
    pub owner_revision: u64,
    pub cursor_revision: u64,
    pub locator_text: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InputLocator {
    pub schema: String,
    pub input_id: String,
    pub task_id: String,
    pub subject_id: String,
    pub owner_revision: u64,
    pub cursor_ref: String,
    pub cursor_revision: u64,
    pub objective_ref: String,
    pub engineering_question: String,
    pub snapshot_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TaskRecord {
    pub task_id: String,
    pub subject_id: String,
    pub kind: SubjectKind,
    pub objective: String,
    pub engineering_question: String,
    pub snapshot_ref: String,
    pub revision: u64,
    pub cursor_revision: u64,
    pub sources: Vec<SourceMaterial>,
    #[serde(default)]
    pub fact_kinds: std::collections::BTreeMap<String, FactKind>,
    pub questions: Vec<Question>,
    pub inputs: Vec<TaskInput>,
    pub completed_effects: Vec<CompletedEffect>,
    pub result: Option<AttributedResult>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TaskCommit {
    pub task_id: String,
    pub owner_revision: u64,
    pub cursor_revision: u64,
    pub digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AdmissionCommand {
    pub command_id: String,
    pub request_digest: String,
    pub command_json: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ControlCommand {
    pub command_id: String,
    pub kind: String,
    pub request_sha256: String,
    pub service_command_json: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OperationEntry {
    pub operation_id: String,
    pub task_id: String,
    pub subject_id: String,
    pub delivery_id: String,
    pub attempt_id: String,
    pub expected_task_revision: u64,
    pub expected_cursor_revision: u64,
    pub grant_ref: String,
    pub grant_revision: u64,
    pub payload_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct QuestionOpenPayload {
    pub question_id: String,
    pub recipient_ref: String,
    pub scope_ref: String,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResultPublishPayload {
    pub result_id: String,
    pub body: String,
    pub source_refs: Vec<SourceReference>,
    pub attribution: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OperationCommit {
    pub operation_id: String,
    pub task_id: String,
    pub kind: String,
    pub owner_revision: u64,
    pub cursor_revision: u64,
    pub digest: String,
}

pub struct TaskStore {
    connection: Mutex<Connection>,
    _writer_lock: File,
    answer_authority_ref: String,
    #[cfg(feature = "controlled-proof")]
    proof_barrier_dir: Option<PathBuf>,
}

impl TaskStore {
    pub fn open(path: &Path, answer_authority_ref: String) -> Result<Self, TaskError> {
        if answer_authority_ref.is_empty() || !path.is_absolute() {
            return Err(TaskError::Invalid);
        }
        let lock_path = path.with_extension("owner.lock");
        let writer_lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(&lock_path)
            .map_err(|_| TaskError::Unavailable)?;
        let lock_meta = writer_lock.metadata().map_err(|_| TaskError::Unavailable)?;
        if !lock_meta.is_file()
            || lock_meta.nlink() != 1
            || lock_meta.permissions().mode() & 0o777 != 0o600
        {
            return Err(TaskError::Invalid);
        }
        writer_lock.try_lock().map_err(|_| TaskError::Busy)?;
        let fresh = match fs::symlink_metadata(path) {
            Ok(_) => false,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
            Err(_) => return Err(TaskError::Unavailable),
        };
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(2))?;
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if !matches!(version, 0 | 2) || (fresh && version != 0) {
            return Err(TaskError::Invalid);
        }
        if !fresh {
            validate_owner_schema(&connection, version == 0)?;
        }
        if version == 0 {
            connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?;
            connection.execute_batch("BEGIN IMMEDIATE;")?;
            let effect_ddl: String = Connection::open_in_memory().and_then(|expected| {
                expected.execute_batch(OWNER_SCHEMA_SQL)?;
                expected.query_row(
                    "SELECT sql FROM sqlite_master WHERE name='completed_effects'",
                    [],
                    |r| r.get(0),
                )
            })?;
            let migration = if fresh {
                Ok(())
            } else {
                rebuild_completed_effects(&connection, &effect_ddl)
            }
            .and_then(|()| {
                connection
                    .execute_batch(OWNER_SCHEMA_SQL)
                    .map_err(TaskError::from)
            })
            .and_then(|()| {
                connection
                    .execute_batch(
                        "INSERT INTO store_identity(singleton, store_id, schema_version)
                       VALUES(1, lower(hex(randomblob(16))), 2);
                     PRAGMA user_version=2;",
                    )
                    .map_err(TaskError::from)
            });
            if let Err(error) = migration {
                let _ = connection.execute_batch("ROLLBACK;");
                return Err(error);
            }
            connection.execute_batch("COMMIT;")?;
        }
        let (store_id, schema_version): (String, i64) = connection.query_row(
            "SELECT store_id, schema_version FROM store_identity WHERE singleton=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if schema_version != 2
            || store_id.len() != 32
            || !store_id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(TaskError::Invalid);
        }
        if version == 2 {
            // Synchronous is per connection; set it only after read-only
            // schema and identity validation above.
            connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?;
        }
        Ok(Self {
            connection: Mutex::new(connection),
            _writer_lock: writer_lock,
            answer_authority_ref,
            #[cfg(feature = "controlled-proof")]
            proof_barrier_dir: None,
        })
    }

    /// Durable identity of this task owner, minted in its own SQLite store.
    pub fn store_id(&self) -> Result<String, TaskError> {
        let connection = self.lock()?;
        let id = connection.query_row(
            "SELECT store_id FROM store_identity WHERE singleton=1 AND schema_version=2",
            [],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    /// Only a newly selected store may be sealed without a prior receipt.
    pub fn is_pristine(&self) -> Result<bool, TaskError> {
        let connection = self.lock()?;
        let count: i64 = connection.query_row(
            "SELECT (SELECT count(*) FROM tasks)
                  + (SELECT count(*) FROM commands)
                  + (SELECT count(*) FROM operations)
                  + (SELECT count(*) FROM completed_effects)
                  + (SELECT count(*) FROM admission_commands)
                  + (SELECT count(*) FROM control_commands)",
            [],
            |row| row.get(0),
        )?;
        Ok(count == 0)
    }

    /// Controlled timing only. The installed host supplies this trusted path;
    /// default builds have no task-lock barrier capability.
    #[cfg(feature = "controlled-proof")]
    pub fn with_controlled_proof_barrier(
        mut self,
        root: Option<PathBuf>,
    ) -> Result<Self, TaskError> {
        if root
            .as_ref()
            .is_some_and(|path| !path.is_absolute() || !path.is_dir())
        {
            return Err(TaskError::Invalid);
        }
        self.proof_barrier_dir = root;
        Ok(self)
    }

    pub fn submit(&self, input: &SubmitTask) -> Result<TaskCommit, TaskError> {
        self.submit_with_status(input).map(|(commit, _)| commit)
    }

    /// The boolean is true only for the first owner publication. A repeated
    /// command must read lifecycle admission back, never be reissued by absence.
    pub fn submit_with_status(&self, input: &SubmitTask) -> Result<(TaskCommit, bool), TaskError> {
        self.submit_inner(input, None)
    }

    /// Commit the owner input and the exact lifecycle admission command as one
    /// owner transaction. The builder receives the locator text made at the
    /// committed owner revision and must not perform external effects.
    pub fn submit_with_admission(
        &self,
        input: &SubmitTask,
        build: &mut AdmissionBuilder<'_>,
    ) -> Result<(TaskCommit, bool), TaskError> {
        self.submit_inner(input, Some(build))
    }

    fn submit_inner(
        &self,
        input: &SubmitTask,
        mut build: Option<&mut AdmissionBuilder<'_>>,
    ) -> Result<(TaskCommit, bool), TaskError> {
        validate_submit(input)?;
        let payload_digest = json_digest(input)?;
        let mut connection = self.lock()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some((kind, digest, task_id, receipt)) = command_row(&tx, &input.command_id)? {
            if kind != "submit" || digest != payload_digest || task_id != input.task_id {
                return Err(TaskError::Conflict);
            }
            return Ok((serde_json::from_slice(&receipt)?, false));
        }
        if let Some(mut record) = read_task(&tx, &input.task_id)? {
            if record.subject_id != input.subject_id
                || record.kind != input.kind
                || record.objective != input.objective
                || record.snapshot_ref != input.snapshot_ref
                || !input.facts.is_empty()
                || record.result.is_some()
            {
                return Err(TaskError::Conflict);
            }
            record.revision += 1;
            let task_input = make_input(&record, &input.command_id, &input.engineering_question)?;
            let admission = build
                .as_mut()
                .map(|build| build(&task_input.locator_text))
                .transpose()?;
            if let Some(command) = &admission {
                validate_admission_for_submit(input, &task_input, command)?;
            }
            record.inputs.push(task_input);
            write_task(&tx, &record)?;
            let commit = commit_of(&record)?;
            tx.execute(
                "INSERT INTO commands VALUES (?1,'submit',?2,?3,?4)",
                params![
                    input.command_id,
                    payload_digest,
                    input.task_id,
                    serde_json::to_vec(&commit)?
                ],
            )?;
            if let Some(command) = admission {
                insert_admission(&tx, &command)?;
            }
            tx.commit()?;
            return Ok((commit, true));
        }
        let mut record = TaskRecord {
            task_id: input.task_id.clone(),
            subject_id: input.subject_id.clone(),
            kind: input.kind,
            objective: input.objective.clone(),
            engineering_question: input.engineering_question.clone(),
            snapshot_ref: input.snapshot_ref.clone(),
            revision: 1,
            cursor_revision: 1,
            sources: Vec::new(),
            fact_kinds: std::collections::BTreeMap::new(),
            questions: Vec::new(),
            inputs: Vec::new(),
            completed_effects: Vec::new(),
            result: None,
        };
        add_source(
            &mut record,
            "objective",
            SourceKind::CanonicalRecord,
            &input.objective,
            true,
        );
        add_source(
            &mut record,
            "engineering-question",
            SourceKind::CanonicalRecord,
            &input.engineering_question,
            true,
        );
        add_source(
            &mut record,
            "snapshot-reference",
            SourceKind::CanonicalRecord,
            &input.snapshot_ref,
            true,
        );
        for fact in &input.facts {
            let (kind, text) = fact_source(fact);
            add_source(&mut record, &fact.id, kind, text, fact.current);
            record.fact_kinds.insert(fact.id.clone(), fact.kind);
        }
        let task_input = make_input(&record, &input.command_id, &input.engineering_question)?;
        let admission = build
            .as_mut()
            .map(|build| build(&task_input.locator_text))
            .transpose()?;
        if let Some(command) = &admission {
            validate_admission_for_submit(input, &task_input, command)?;
        }
        record.inputs.push(task_input);
        write_task(&tx, &record)?;
        let commit = commit_of(&record)?;
        tx.execute(
            "INSERT INTO commands VALUES (?1,'submit',?2,?3,?4)",
            params![
                input.command_id,
                payload_digest,
                input.task_id,
                serde_json::to_vec(&commit)?
            ],
        )?;
        if let Some(command) = admission {
            insert_admission(&tx, &command)?;
        }
        tx.commit()?;
        Ok((commit, true))
    }

    pub fn answer(&self, input: &AnswerQuestion) -> Result<TaskCommit, TaskError> {
        if !valid_id(&input.command_id)
            || !valid_id(&input.question_id)
            || !valid_text(&input.answer)
            || input.expected_revision == 0
        {
            return Err(TaskError::Invalid);
        }
        if input.authority_ref != self.answer_authority_ref {
            return Err(TaskError::Unauthorized);
        }
        let payload_digest = json_digest(input)?;
        let mut connection = self.lock()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some((kind, digest, task_id, receipt)) = command_row(&tx, &input.command_id)? {
            if kind != "answer" || digest != payload_digest || task_id != input.task_id {
                return Err(TaskError::Conflict);
            }
            return Ok(serde_json::from_slice(&receipt)?);
        }
        let mut record = read_task(&tx, &input.task_id)?.ok_or(TaskError::Unavailable)?;
        if record.revision != input.expected_revision {
            return Err(TaskError::Stale);
        }
        let question = record
            .questions
            .iter_mut()
            .find(|q| q.id == input.question_id)
            .ok_or(TaskError::Unavailable)?;
        if question.recipient_ref != input.recipient_ref
            || question.scope_ref != input.scope_ref
            || !matches!(question.state, QuestionState::Pending)
        {
            return Err(TaskError::Conflict);
        }
        question.state = QuestionState::Answered {
            answer: input.answer.clone(),
            authority_ref: input.authority_ref.clone(),
        };
        record.revision += 1;
        record.cursor_revision += 1;
        update_source(
            &mut record,
            &format!("question-{}", input.question_id),
            &format!(
                "Question {} answered by {} in scope {}: {}",
                input.question_id, input.authority_ref, input.scope_ref, input.answer
            ),
        )?;
        write_task(&tx, &record)?;
        let commit = commit_of(&record)?;
        tx.execute(
            "INSERT INTO commands VALUES (?1,'answer',?2,?3,?4)",
            params![
                input.command_id,
                payload_digest,
                input.task_id,
                serde_json::to_vec(&commit)?
            ],
        )?;
        tx.commit()?;
        Ok(commit)
    }

    pub fn observe(&self, task_id: &str) -> Result<Option<TaskRecord>, TaskError> {
        let connection = self.lock()?;
        read_task(&connection, task_id)
    }

    pub fn input_text(&self, command_id: &str) -> Result<Option<String>, TaskError> {
        let connection = self.lock()?;
        let Some((kind, _, task_id, _)) = command_row(&connection, command_id)? else {
            return Ok(None);
        };
        if kind != "submit" {
            return Ok(None);
        }
        let record = read_task(&connection, &task_id)?.ok_or(TaskError::Uncertain)?;
        Ok(record
            .inputs
            .iter()
            .find(|input| input.command_id == command_id)
            .map(|input| input.locator_text.clone()))
    }

    /// Read an exact, already committed owner command without opening a write
    /// transaction. Recovery-only hosts use this before refusing new work.
    pub fn lookup_command_receipt(
        &self,
        kind: &str,
        task_id: &str,
        command_id: &str,
        payload_digest: &str,
    ) -> Result<Option<TaskCommit>, TaskError> {
        if !matches!(kind, "submit" | "answer") || !valid_digest(payload_digest) {
            return Err(TaskError::Invalid);
        }
        let connection = self.lock()?;
        let Some((stored_kind, stored_digest, stored_task_id, receipt)) =
            command_row(&connection, command_id)?
        else {
            return Ok(None);
        };
        if stored_kind != kind || stored_digest != payload_digest || stored_task_id != task_id {
            return Err(TaskError::Conflict);
        }
        Ok(Some(serde_json::from_slice(&receipt)?))
    }

    pub fn record_admission_command(&self, command: &AdmissionCommand) -> Result<(), TaskError> {
        if !valid_id(&command.command_id)
            || !valid_digest(&command.request_digest)
            || command.command_json.len() > MAX_TEXT
        {
            return Err(TaskError::Invalid);
        }
        let mut connection = self.lock()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some((kind, _, _, _)) = command_row(&tx, &command.command_id)? else {
            return Err(TaskError::Unavailable);
        };
        if kind != "submit" {
            return Err(TaskError::Invalid);
        }
        let old: Option<(String, String)> = tx
            .query_row(
                "SELECT request_digest,command_json FROM admission_commands WHERE command_id=?1",
                [&command.command_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some(old) = old {
            if old != (command.request_digest.clone(), command.command_json.clone()) {
                return Err(TaskError::Conflict);
            }
            return Ok(());
        }
        tx.execute(
            "INSERT INTO admission_commands VALUES (?1,?2,?3)",
            params![
                command.command_id,
                command.request_digest,
                command.command_json
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn lookup_admission_command(
        &self,
        command_id: &str,
    ) -> Result<Option<AdmissionCommand>, TaskError> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT request_digest,command_json FROM admission_commands WHERE command_id=?1",
                [command_id],
                |r| {
                    Ok(AdmissionCommand {
                        command_id: command_id.into(),
                        request_digest: r.get(0)?,
                        command_json: r.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(TaskError::from)
    }

    pub fn admission_commands(&self) -> Result<Vec<AdmissionCommand>, TaskError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT a.command_id,a.request_digest,a.command_json,c.kind \
             FROM admission_commands a LEFT JOIN commands c ON c.command_id=a.command_id \
             ORDER BY a.command_id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                AdmissionCommand {
                    command_id: row.get(0)?,
                    request_digest: row.get(1)?,
                    command_json: row.get(2)?,
                },
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        let mut commands = Vec::new();
        for row in rows {
            let (command, kind) = row?;
            if kind.as_deref() != Some("submit") {
                return Err(TaskError::Invalid);
            }
            commands.push(command);
        }
        Ok(commands)
    }

    pub fn record_control_command(&self, command: &ControlCommand) -> Result<bool, TaskError> {
        if !valid_id(&command.command_id)
            || !valid_digest(&command.request_sha256)
            || command.service_command_json.len() > MAX_TEXT
            || !matches!(command.kind.as_str(), "replacement" | "contract_grant")
        {
            return Err(TaskError::Invalid);
        }
        let mut connection = self.lock()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let prior:Option<(String,String,String)>=tx.query_row("SELECT kind,request_sha256,service_command_json FROM control_commands WHERE command_id=?1",[&command.command_id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        if let Some(prior) = prior {
            if prior
                != (
                    command.kind.clone(),
                    command.request_sha256.clone(),
                    command.service_command_json.clone(),
                )
            {
                return Err(TaskError::Conflict);
            }
            return Ok(false);
        }
        tx.execute(
            "INSERT INTO control_commands VALUES (?1,?2,?3,?4)",
            params![
                command.command_id,
                command.kind,
                command.request_sha256,
                command.service_command_json
            ],
        )?;
        tx.commit()?;
        Ok(true)
    }
    pub fn lookup_control_command(
        &self,
        command_id: &str,
    ) -> Result<Option<ControlCommand>, TaskError> {
        let connection = self.lock()?;
        connection.query_row("SELECT kind,request_sha256,service_command_json FROM control_commands WHERE command_id=?1",[command_id],|r|Ok(ControlCommand{command_id:command_id.into(),kind:r.get(0)?,request_sha256:r.get(1)?,service_command_json:r.get(2)?})).optional().map_err(TaskError::from)
    }

    pub fn validate_input(&self, input_id: &str, text: &str) -> Result<InputLocator, TaskError> {
        if text.len() > 8 * 1024 {
            return Err(TaskError::Invalid);
        }
        let locator: InputLocator = serde_json::from_str(text)?;
        if locator.schema != "retained-task-input-v1"
            || locator.input_id != input_id
            || locator.cursor_ref != locator.task_id
            || locator.objective_ref != "objective"
        {
            return Err(TaskError::Invalid);
        }
        let connection = self.lock()?;
        let record = read_task(&connection, &locator.task_id)?.ok_or(TaskError::Unavailable)?;
        if record.subject_id != locator.subject_id || record.snapshot_ref != locator.snapshot_ref {
            return Err(TaskError::Conflict);
        }
        let input = record
            .inputs
            .iter()
            .find(|input| input.input_id == input_id)
            .ok_or(TaskError::Unavailable)?;
        if input.locator_text != text
            || input.owner_revision != locator.owner_revision
            || input.cursor_revision != locator.cursor_revision
            || input.engineering_question != locator.engineering_question
        {
            return Err(TaskError::Conflict);
        }
        Ok(locator)
    }

    pub fn observe_subject(&self, subject_id: &str) -> Result<Option<TaskRecord>, TaskError> {
        let connection = self.lock()?;
        let value: Option<Vec<u8>> = connection
            .query_row(
                "SELECT record_json FROM tasks WHERE subject_id=?1",
                [subject_id],
                |row| row.get(0),
            )
            .optional()?;
        value
            .map(|bytes| serde_json::from_slice(&bytes).map_err(TaskError::from))
            .transpose()
    }

    /// Invoked only after lifecycle's exact child/effect settlement is committed.
    /// The owner record, not this callback's return, is durable cursor evidence.
    pub fn record_completed_effect(
        &self,
        task_id: &str,
        effect: CompletedEffect,
        expected_cursor_revision: u64,
    ) -> Result<TaskCommit, TaskError> {
        if !valid_id(&effect.effect_id)
            || !valid_id(&effect.delivery_id)
            || !valid_digest(&effect.receipt_digest)
        {
            return Err(TaskError::Invalid);
        }
        let attribution = effect.attribution.as_ref().ok_or(TaskError::Invalid)?;
        validate_effect_attribution(attribution, &effect.receipt_digest)?;
        let mut connection = self.lock()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<CompletedEffectRow> = tx.query_row(
            "SELECT task_id,delivery_id,receipt_digest,owner_revision,cursor_revision,record_digest FROM completed_effects WHERE effect_id=?1",
            [&effect.effect_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).optional()?;
        if let Some(existing) = existing {
            if (
                existing.0.as_str(),
                existing.1.as_str(),
                existing.2.as_str(),
            ) != (
                task_id,
                effect.delivery_id.as_str(),
                effect.receipt_digest.as_str(),
            ) {
                return Err(TaskError::Conflict);
            }
            let record = read_task(&tx, task_id)?.ok_or(TaskError::Uncertain)?;
            let recorded = record
                .completed_effects
                .iter()
                .find(|item| item.effect_id == effect.effect_id)
                .ok_or(TaskError::Uncertain)?;
            if recorded.delivery_id != effect.delivery_id
                || recorded.receipt_digest != effect.receipt_digest
                || (recorded.attribution.is_some() && recorded.attribution != effect.attribution)
            {
                return Err(TaskError::Conflict);
            }
            return match stored_effect_receipt(
                task_id, recorded, existing.3, existing.4, existing.5,
            )? {
                Some(receipt) => Ok(receipt),
                None => commit_of(&record), // Migrated v1 has no historical receipt.
            };
        }
        let mut record = read_task(&tx, task_id)?.ok_or(TaskError::Unavailable)?;
        if record.cursor_revision != expected_cursor_revision {
            return Err(TaskError::Stale);
        }
        record.completed_effects.push(effect.clone());
        record.revision += 1;
        record.cursor_revision += 1;
        append_effect_sources(&mut record, &effect)?;
        let receipt = commit_of(&record)?;
        write_task(&tx, &record)?;
        tx.execute(
            "INSERT INTO completed_effects(effect_id,task_id,delivery_id,receipt_digest,owner_revision,cursor_revision,record_digest) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                effect.effect_id,
                task_id,
                effect.delivery_id,
                effect.receipt_digest,
                i64::try_from(receipt.owner_revision).map_err(|_| TaskError::Invalid)?,
                i64::try_from(receipt.cursor_revision).map_err(|_| TaskError::Invalid)?,
                receipt.digest,
            ],
        )?;
        #[cfg(feature = "controlled-proof")]
        self.controlled_barrier("snapshot_before_owner_commit")?;
        tx.commit()?;
        Ok(receipt)
    }

    pub fn lookup_completed_effect(
        &self,
        effect_id: &str,
    ) -> Result<Option<CompletedEffect>, TaskError> {
        let connection = self.lock()?;
        let row: Option<CompletedEffectRow> = connection.query_row(
            "SELECT task_id,delivery_id,receipt_digest,owner_revision,cursor_revision,record_digest FROM completed_effects WHERE effect_id=?1",
            [effect_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?)),
        ).optional()?;
        let Some((
            task_id,
            delivery_id,
            receipt_digest,
            owner_revision,
            cursor_revision,
            record_digest,
        )) = row
        else {
            return Ok(None);
        };
        let record = read_task(&connection, &task_id)?.ok_or(TaskError::Uncertain)?;
        let effect = record
            .completed_effects
            .into_iter()
            .find(|item| item.effect_id == effect_id)
            .ok_or(TaskError::Uncertain)?;
        if effect.delivery_id != delivery_id || effect.receipt_digest != receipt_digest {
            return Err(TaskError::Conflict);
        }
        stored_effect_receipt(
            &task_id,
            &effect,
            owner_revision,
            cursor_revision,
            record_digest,
        )?;
        Ok(Some(effect))
    }

    pub fn lookup_operation(
        &self,
        operation_id: &str,
    ) -> Result<Option<OperationCommit>, TaskError> {
        let connection = self.lock()?;
        let bytes: Option<Vec<u8>> = connection
            .query_row(
                "SELECT commit_json FROM operations WHERE operation_id=?1",
                [operation_id],
                |r| r.get(0),
            )
            .optional()?;
        bytes
            .map(|bytes| serde_json::from_slice(&bytes).map_err(TaskError::from))
            .transpose()
    }

    /// `with_grant_fence` must acquire and hold the lifecycle grant/currentness
    /// transaction while it executes its callback. Lock order is task then lifecycle.
    pub fn open_question(
        &self,
        entry: &OperationEntry,
        payload: &QuestionOpenPayload,
        with_grant_fence: &mut TaskGrantFence<'_>,
    ) -> Result<OperationCommit, TaskError> {
        self.commit_operation(
            entry,
            "task.question.open.v1",
            payload,
            with_grant_fence,
            |record| {
                if !valid_id(&payload.question_id)
                    || !valid_id(&payload.recipient_ref)
                    || !valid_id(&payload.scope_ref)
                    || !valid_text(&payload.text)
                    || record.questions.iter().any(|q| q.id == payload.question_id)
                {
                    return Err(TaskError::Invalid);
                }
                record.questions.push(Question {
                    id: payload.question_id.clone(),
                    recipient_ref: payload.recipient_ref.clone(),
                    scope_ref: payload.scope_ref.clone(),
                    text: payload.text.clone(),
                    state: QuestionState::Pending,
                });
                add_source(
                    record,
                    &format!("question-{}", payload.question_id),
                    SourceKind::HumanInteraction,
                    &format!(
                        "Pending question {} to {} in scope {}: {}",
                        payload.question_id, payload.recipient_ref, payload.scope_ref, payload.text
                    ),
                    true,
                );
                Ok(())
            },
        )
    }

    pub fn publish_result(
        &self,
        entry: &OperationEntry,
        payload: &ResultPublishPayload,
        with_grant_fence: &mut TaskGrantFence<'_>,
    ) -> Result<OperationCommit, TaskError> {
        self.commit_operation(
            entry,
            "task.result.publish.v1",
            payload,
            with_grant_fence,
            |record| {
                if !valid_id(&payload.result_id)
                    || !valid_text(&payload.body)
                    || !valid_id(&payload.attribution)
                    || record.result.is_some()
                {
                    return Err(TaskError::Invalid);
                }
                if record
                    .questions
                    .iter()
                    .any(|q| matches!(q.state, QuestionState::Pending))
                {
                    return Err(TaskError::PendingQuestion);
                }
                if payload.source_refs.is_empty()
                    || payload.source_refs.iter().any(|reference| {
                        !record.sources.iter().any(|source| {
                            source.current
                                && source.availability == SourceAvailability::Complete
                                && source.reference() == *reference
                        })
                    })
                {
                    return Err(TaskError::Stale);
                }
                record.result = Some(AttributedResult {
                    result_id: payload.result_id.clone(),
                    operation_id: entry.operation_id.clone(),
                    body: payload.body.clone(),
                    source_refs: payload.source_refs.clone(),
                    attribution: payload.attribution.clone(),
                });
                add_source(
                    record,
                    &format!("result-{}", payload.result_id),
                    SourceKind::DeliveryConsequence,
                    &payload.body,
                    true,
                );
                Ok(())
            },
        )
    }

    fn commit_operation<P: Serialize>(
        &self,
        entry: &OperationEntry,
        kind: &str,
        payload: &P,
        with_grant_fence: &mut TaskGrantFence<'_>,
        update: impl FnOnce(&mut TaskRecord) -> Result<(), TaskError>,
    ) -> Result<OperationCommit, TaskError> {
        if !valid_id(&entry.operation_id)
            || !valid_id(&entry.task_id)
            || !valid_id(&entry.delivery_id)
            || !valid_id(&entry.attempt_id)
            || !valid_id(&entry.grant_ref)
            || entry.grant_revision == 0
        {
            return Err(TaskError::Invalid);
        }
        let digest = json_digest(payload)?;
        let entry_digest = json_digest(entry)?;
        if digest != entry.payload_sha256 {
            return Err(TaskError::Conflict);
        }
        let mut connection = self.lock()?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<(String, String, String, Vec<u8>)> = tx
            .query_row(
                "SELECT kind,payload_digest,entry_digest,commit_json FROM operations WHERE operation_id=?1",
                [&entry.operation_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?,r.get(3)?)),
            )
            .optional()?;
        if let Some((old_kind, old_digest, old_entry_digest, bytes)) = existing {
            if old_kind != kind || old_digest != digest || old_entry_digest != entry_digest {
                return Err(TaskError::Conflict);
            }
            return Ok(serde_json::from_slice(&bytes)?);
        }
        let mut record = read_task(&tx, &entry.task_id)?.ok_or(TaskError::Unavailable)?;
        if record.subject_id != entry.subject_id
            || record.revision != entry.expected_task_revision
            || record.cursor_revision != entry.expected_cursor_revision
        {
            return Err(TaskError::Stale);
        }
        update(&mut record)?;
        record.revision += 1;
        record.cursor_revision += 1;
        let receipt = OperationCommit {
            operation_id: entry.operation_id.clone(),
            task_id: entry.task_id.clone(),
            kind: kind.into(),
            owner_revision: record.revision,
            cursor_revision: record.cursor_revision,
            digest: json_digest(&record)?,
        };
        #[cfg(feature = "controlled-proof")]
        if kind == "task.result.publish.v1" {
            self.controlled_barrier("task_result_after_owner_begin")?;
        }
        let mut tx = Some(tx);
        with_grant_fence(&mut || {
            let tx = tx.take().ok_or(TaskError::Uncertain)?;
            write_task(&tx, &record)?;
            tx.execute(
                "INSERT INTO operations VALUES (?1,?2,?3,?4,?5)",
                params![
                    entry.operation_id,
                    kind,
                    digest,
                    entry_digest,
                    serde_json::to_vec(&receipt)?
                ],
            )?;
            tx.commit()?;
            Ok(())
        })?;
        if tx.is_some() {
            return Err(TaskError::Uncertain);
        }
        Ok(receipt)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, TaskError> {
        self.connection.lock().map_err(|_| TaskError::Uncertain)
    }

    #[cfg(feature = "controlled-proof")]
    fn controlled_barrier(&self, point: &str) -> Result<(), TaskError> {
        let Some(root) = self.proof_barrier_dir.as_deref() else {
            return Ok(());
        };
        if !root.join(format!("{point}.arm")).exists() {
            return Ok(());
        }
        let reached = root.join(format!("{point}.reached"));
        let release = root.join(format!("{point}.release"));
        let mut file = std::fs::File::create(reached).map_err(|_| TaskError::Uncertain)?;
        use std::io::Write as _;
        file.write_all(b"reached\n")
            .map_err(|_| TaskError::Uncertain)?;
        file.sync_all().map_err(|_| TaskError::Uncertain)?;
        let deadline = Instant::now() + Duration::from_secs(15);
        while !release.exists() {
            if Instant::now() >= deadline {
                return Err(TaskError::Busy);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }
}

fn read_task(connection: &Connection, task_id: &str) -> Result<Option<TaskRecord>, TaskError> {
    let bytes: Option<Vec<u8>> = connection
        .query_row(
            "SELECT record_json FROM tasks WHERE task_id=?1",
            [task_id],
            |r| r.get(0),
        )
        .optional()?;
    bytes
        .map(|bytes| serde_json::from_slice(&bytes).map_err(TaskError::from))
        .transpose()
}
fn write_task(connection: &Connection, record: &TaskRecord) -> Result<(), TaskError> {
    connection.execute("INSERT INTO tasks(task_id,subject_id,record_json) VALUES(?1,?2,?3) ON CONFLICT(task_id) DO UPDATE SET record_json=excluded.record_json",params![record.task_id,record.subject_id,serde_json::to_vec(record)?])?;
    Ok(())
}
fn command_row(connection: &Connection, id: &str) -> Result<Option<CommandRow>, TaskError> {
    connection
        .query_row(
            "SELECT kind,payload_digest,task_id,commit_json FROM commands WHERE command_id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
        .map_err(TaskError::from)
}
fn commit_of(record: &TaskRecord) -> Result<TaskCommit, TaskError> {
    Ok(TaskCommit {
        task_id: record.task_id.clone(),
        owner_revision: record.revision,
        cursor_revision: record.cursor_revision,
        digest: json_digest(record)?,
    })
}

fn stored_effect_receipt(
    task_id: &str,
    effect: &CompletedEffect,
    owner_revision: Option<i64>,
    cursor_revision: Option<i64>,
    record_digest: Option<String>,
) -> Result<Option<TaskCommit>, TaskError> {
    match (
        effect.attribution.as_ref(),
        owner_revision,
        cursor_revision,
        record_digest,
    ) {
        (None, None, None, None) => Ok(None),
        (Some(attribution), Some(owner), Some(cursor), Some(digest))
            if owner > 0 && cursor > 0 && valid_digest(&digest) =>
        {
            validate_effect_attribution(attribution, &effect.receipt_digest)
                .map_err(|_| TaskError::Uncertain)?;
            Ok(Some(TaskCommit {
                task_id: task_id.into(),
                owner_revision: u64::try_from(owner).map_err(|_| TaskError::Uncertain)?,
                cursor_revision: u64::try_from(cursor).map_err(|_| TaskError::Uncertain)?,
                digest,
            }))
        }
        _ => Err(TaskError::Uncertain),
    }
}
fn insert_admission(
    tx: &rusqlite::Transaction<'_>,
    command: &AdmissionCommand,
) -> Result<(), TaskError> {
    tx.execute(
        "INSERT INTO admission_commands VALUES (?1,?2,?3)",
        params![
            command.command_id,
            command.request_digest,
            command.command_json
        ],
    )?;
    Ok(())
}

fn validate_admission_for_submit(
    submit: &SubmitTask,
    input: &TaskInput,
    command: &AdmissionCommand,
) -> Result<(), TaskError> {
    if command.command_id != submit.command_id
        || !valid_digest(&command.request_digest)
        || command.command_json.len() > MAX_TEXT
    {
        return Err(TaskError::Invalid);
    }
    let dto: serde_json::Value = serde_json::from_str(&command.command_json)?;
    let object = dto.as_object().ok_or(TaskError::Invalid)?;
    const FIELDS: &[&str] = &[
        "protocol_version",
        "principal_ref",
        "command_id",
        "subject_id",
        "context_generation",
        "build_id",
        "proof_run_id",
        "grant_ref",
        "expected_revision",
        "request_digest",
        "kind",
        "payload",
    ];
    if object.len() != FIELDS.len()
        || object.keys().any(|key| !FIELDS.contains(&key.as_str()))
        || dto["protocol_version"].as_u64() != Some(2)
        || dto["command_id"].as_str() != Some(submit.command_id.as_str())
        || dto["subject_id"].as_str() != Some(submit.subject_id.as_str())
        || dto["request_digest"].as_str() != Some(command.request_digest.as_str())
        || dto["kind"].as_str() != Some("enqueue_input")
        || dto["expected_revision"]
            .as_str()
            .and_then(|value| value.parse::<u64>().ok())
            .is_none()
    {
        return Err(TaskError::Invalid);
    }
    let payload = dto["payload"].as_object().ok_or(TaskError::Invalid)?;
    const PAYLOAD: &[&str] = &["input_id", "producer_ref", "text", "text_digest"];
    if payload.len() != PAYLOAD.len()
        || payload.keys().any(|key| !PAYLOAD.contains(&key.as_str()))
        || dto["payload"]["input_id"].as_str() != Some(input.input_id.as_str())
        || dto["payload"]["text"].as_str() != Some(input.locator_text.as_str())
        || !dto["payload"]["text_digest"]
            .as_str()
            .is_some_and(valid_digest)
    {
        return Err(TaskError::Invalid);
    }
    Ok(())
}

fn make_input(
    record: &TaskRecord,
    command_id: &str,
    question: &str,
) -> Result<TaskInput, TaskError> {
    let input_id = format!("input:{command_id}");
    let locator = InputLocator {
        schema: "retained-task-input-v1".into(),
        input_id: input_id.clone(),
        task_id: record.task_id.clone(),
        subject_id: record.subject_id.clone(),
        owner_revision: record.revision,
        cursor_ref: record.task_id.clone(),
        cursor_revision: record.cursor_revision,
        objective_ref: "objective".into(),
        engineering_question: question.into(),
        snapshot_ref: record.snapshot_ref.clone(),
    };
    let locator_text = serde_json::to_string(&locator)?;
    if locator_text.len() > 8 * 1024 {
        return Err(TaskError::Invalid);
    }
    Ok(TaskInput {
        command_id: command_id.into(),
        input_id,
        engineering_question: question.into(),
        owner_revision: record.revision,
        cursor_revision: record.cursor_revision,
        locator_text,
    })
}
fn add_source(record: &mut TaskRecord, id: &str, kind: SourceKind, text: &str, required: bool) {
    let trust = if kind == SourceKind::Governing {
        SourceTrust::HostGoverning
    } else {
        SourceTrust::HostCanonical
    };
    record.sources.push(SourceMaterial {
        id: id.into(),
        owner_namespace: OWNER_NAMESPACE.into(),
        owner_ref: record.task_id.clone(),
        revision: 1,
        dependency_key: format!("{}:{id}", record.task_id),
        current: required,
        dependency_required: required,
        kind,
        trust,
        availability: SourceAvailability::Complete,
        bytes: text.as_bytes().to_vec(),
        sha256: sha256(text.as_bytes()),
    });
}

fn validate_effect_attribution(
    a: &CompletedSnapshotReadAttribution,
    receipt_digest: &str,
) -> Result<(), TaskError> {
    if a.schema != "completed-snapshot-read-attribution-v2"
        || a.contract_id != "workspace.snapshot.read.v1"
        || !valid_id(&a.parent_invocation_id)
        || !valid_id(&a.call_id)
        || !valid_id(&a.snapshot_id)
        || !valid_id(&a.member_id)
        || a.requested_length == 0
        || a.requested_length > 65_536
        // The current native contract refuses a range past member end and
        // returns exactly the requested slice. Keep both fields explicit.
        || a.returned_length != a.requested_length
        || a.offset.checked_add(a.requested_length).is_none()
        || !valid_digest(&a.result_sha256)
        || a.result_sha256 != receipt_digest
    {
        return Err(TaskError::Invalid);
    }
    match &a.result {
        CompletedSnapshotResult::InlineUtf8 { text } => {
            if text.len() as u64 != a.returned_length
                || text.len() > EFFECT_INLINE_BYTES
                || serde_json::to_vec(text)?.len() > EFFECT_ESCAPED_BYTES
                || sha256(text.as_bytes()) != receipt_digest
            {
                return Err(TaskError::Invalid);
            }
        }
        CompletedSnapshotResult::Unavailable { .. } => {}
    }
    Ok(())
}

fn append_effect_sources(
    record: &mut TaskRecord,
    effect: &CompletedEffect,
) -> Result<(), TaskError> {
    let a = effect.attribution.as_ref().ok_or(TaskError::Invalid)?;
    let id_hash = sha256(effect.effect_id.as_bytes());
    let occurrence_id = format!("effect-{id_hash}");
    let data_id = format!("effect-data-{id_hash}");
    if record
        .sources
        .iter()
        .any(|s| s.id == occurrence_id || s.id == data_id)
    {
        return Err(TaskError::Conflict);
    }
    let (availability, bytes, reason) = match &a.result {
        CompletedSnapshotResult::InlineUtf8 { text } => {
            (SourceAvailability::Complete, text.as_bytes().to_vec(), None)
        }
        CompletedSnapshotResult::Unavailable { reason } => {
            (SourceAvailability::Unavailable, Vec::new(), Some(reason))
        }
    };
    let occurrence = serde_json::to_vec(&serde_json::json!({
        "version":2, "effect_id":effect.effect_id, "delivery_id":effect.delivery_id,
        "parent_invocation_id":a.parent_invocation_id, "call_id":a.call_id,
        "contract_id":a.contract_id, "snapshot_id":a.snapshot_id,
        "member_id":a.member_id, "offset":a.offset,
        "requested_length":a.requested_length, "returned_length":a.returned_length,
        "result_sha256":a.result_sha256, "result_source_id":data_id,
        "result_inline":reason.is_none(), "unavailable_reason":reason,
    }))?;
    let make =
        |id: String, kind, trust, availability, bytes: Vec<u8>, digest: String| SourceMaterial {
            dependency_key: id.clone(),
            id,
            owner_namespace: OWNER_NAMESPACE.into(),
            owner_ref: record.task_id.clone(),
            revision: 1,
            current: true,
            dependency_required: true,
            kind,
            trust,
            availability,
            bytes,
            sha256: digest,
        };
    record.sources.push(make(
        occurrence_id,
        SourceKind::DeliveryConsequence,
        SourceTrust::HostCanonical,
        SourceAvailability::Complete,
        occurrence.clone(),
        sha256(&occurrence),
    ));
    record.sources.push(make(
        data_id,
        SourceKind::ExternalData,
        SourceTrust::UntrustedData,
        availability,
        bytes,
        effect.receipt_digest.clone(),
    ));
    Ok(())
}
fn update_source(record: &mut TaskRecord, id: &str, text: &str) -> Result<(), TaskError> {
    let source = record
        .sources
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or(TaskError::Unavailable)?;
    source.revision += 1;
    source.bytes = text.as_bytes().to_vec();
    source.sha256 = sha256(&source.bytes);
    Ok(())
}
fn fact_source(fact: &FactInput) -> (SourceKind, &str) {
    let kind = match fact.kind {
        FactKind::Governing => SourceKind::Governing,
        FactKind::CanonicalRecord | FactKind::CanonicalReference => SourceKind::CanonicalRecord,
        FactKind::Decision => SourceKind::Decision,
        FactKind::Commitment => SourceKind::CanonicalRecord,
        FactKind::Uncertainty => SourceKind::CanonicalRecord,
        FactKind::HumanInteraction => SourceKind::HumanInteraction,
    };
    (kind, &fact.text)
}
fn default_current() -> bool {
    true
}
fn validate_submit(input: &SubmitTask) -> Result<(), TaskError> {
    if !valid_id(&input.command_id)
        || input.command_id.len() > 120
        || !valid_id(&input.task_id)
        || !valid_id(&input.subject_id)
        || !valid_id(&input.snapshot_ref)
        || !valid_text(&input.objective)
        || !valid_text(&input.engineering_question)
        || input.facts.len() > 32
        || input.objective.len()
            + input.engineering_question.len()
            + input.snapshot_ref.len()
            + input.facts.iter().map(|f| f.text.len()).sum::<usize>()
            > 64 * 1024
        || input.facts.iter().any(|f| {
            !valid_id(&f.id)
                || !valid_text(&f.text)
                || ["objective", "engineering-question", "snapshot-reference"]
                    .contains(&f.id.as_str())
                || ["question-", "effect-", "result-", "input-"]
                    .iter()
                    .any(|prefix| f.id.starts_with(prefix))
        })
    {
        return Err(TaskError::Invalid);
    }
    let mut ids = std::collections::BTreeSet::new();
    if input.facts.iter().any(|f| !ids.insert(&f.id)) {
        return Err(TaskError::Invalid);
    }
    Ok(())
}
fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b))
}
fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_TEXT && !value.contains('\0')
}
fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
pub fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn json_digest<T: Serialize>(value: &T) -> Result<String, TaskError> {
    Ok(sha256(&serde_json::to_vec(value)?))
}

pub fn obligations(record: &TaskRecord) -> Vec<RequiredObligation> {
    record
        .sources
        .iter()
        .filter(|s| s.current)
        .map(|s| {
            let kind = match s.id.as_str() {
                "objective" | "engineering-question" => ObligationKind::Objective,
                "snapshot-reference" => ObligationKind::CanonicalReference,
                _ if s.id.starts_with("question-") => ObligationKind::HumanInteraction,
                _ if s.id.starts_with("effect-") => ObligationKind::CompletedConsequence,
                _ if s.id.starts_with("result-") => ObligationKind::CompletedConsequence,
                _ => match record.fact_kinds.get(&s.id) {
                    Some(FactKind::Governing) => ObligationKind::Governing,
                    Some(FactKind::Decision) => ObligationKind::Decision,
                    Some(FactKind::Commitment) => ObligationKind::Commitment,
                    Some(FactKind::Uncertainty) => ObligationKind::Uncertainty,
                    Some(FactKind::HumanInteraction) => ObligationKind::HumanInteraction,
                    _ => ObligationKind::CanonicalReference,
                },
            };
            RequiredObligation {
                id: format!("obligation-{}", s.id),
                kind,
                verifier_scope: format!("Preserve current {} source and its consequence", s.id),
                description: format!("Current task-owned {} remains represented accurately", s.id),
                sources: vec![s.reference()],
            }
        })
        .collect()
}

#[cfg(test)]
mod schema_two_tests {
    use super::*;

    #[test]
    fn failed_explicit_migration_rolls_back_identity_ddl_and_user_version() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("owner.sqlite");
        let store = TaskStore::open(&path, "human-operator".into()).unwrap();
        let id = store.store_id().unwrap();
        store
            .submit(&SubmitTask {
                command_id: "rollback-command".into(),
                task_id: "rollback-task".into(),
                subject_id: "rollback-subject".into(),
                kind: SubjectKind::RepositoryAnalysis,
                objective: "Preserve the original owner record".into(),
                engineering_question: "Does a failed migration preserve it?".into(),
                snapshot_ref: "rollback-snapshot".into(),
                facts: Vec::new(),
            })
            .unwrap();
        drop(store);
        let db = Connection::open(&path).unwrap();
        let v2_ddl: String = db
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name='store_identity'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let v1_ddl = v2_ddl.replace("schema_version = 2", "schema_version = 1");
        db.execute_batch(
            "BEGIN IMMEDIATE; DROP TABLE store_identity; DROP TABLE completed_effects;
            CREATE TABLE completed_effects (
                effect_id TEXT PRIMARY KEY, task_id TEXT NOT NULL,
                delivery_id TEXT NOT NULL, receipt_digest TEXT NOT NULL
            );",
        )
        .unwrap();
        db.execute_batch(&v1_ddl).unwrap();
        db.execute("INSERT INTO store_identity VALUES(1,?1,1)", [&id])
            .unwrap();
        db.execute_batch("PRAGMA user_version=1; COMMIT;").unwrap();
        db.execute("INSERT INTO completed_effects VALUES('legacy-effect','rollback-task','legacy-delivery','legacy-digest')",[]).unwrap();
        let effect_ddl: String = db
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name='completed_effects'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let effect_row: (String, String, String, String) = db
            .query_row(
                "SELECT effect_id,task_id,delivery_id,receipt_digest FROM completed_effects",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        let original_record: Vec<u8> = db
            .query_row(
                "SELECT record_json FROM tasks WHERE task_id='rollback-task'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let original_command: Vec<u8> = db
            .query_row(
                "SELECT commit_json FROM commands WHERE command_id='rollback-command'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        drop(db);
        assert!(matches!(
            migrate_v1_to_v2_inner(&path, true),
            Err(TaskError::Invalid)
        ));
        let db = Connection::open(&path).unwrap();
        let version: i64 = db
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        let (after_id, after_version): (String, i64) = db
            .query_row(
                "SELECT store_id,schema_version FROM store_identity WHERE singleton=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let after_ddl: String = db
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name='store_identity'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            (version, after_id, after_version, after_ddl),
            (1, id, 1, v1_ddl)
        );
        let after_effect_ddl: String = db
            .query_row(
                "SELECT sql FROM sqlite_master WHERE name='completed_effects'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let after_effect_row: (String, String, String, String) = db
            .query_row(
                "SELECT effect_id,task_id,delivery_id,receipt_digest FROM completed_effects",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!(
            (after_effect_ddl, after_effect_row),
            (effect_ddl, effect_row)
        );
        let after_record: Vec<u8> = db
            .query_row(
                "SELECT record_json FROM tasks WHERE task_id='rollback-task'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let after_command: Vec<u8> = db
            .query_row(
                "SELECT commit_json FROM commands WHERE command_id='rollback-command'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            (after_record, after_command),
            (original_record, original_command)
        );
    }
}
