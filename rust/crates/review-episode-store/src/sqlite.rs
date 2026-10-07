use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::Duration;

use review_episode_core::state::ReviewEpisodeState;
use rusqlite::{
    Connection, Error as SqlError, ErrorCode, OpenFlags, OptionalExtension, TransactionBehavior,
    params,
};

use crate::integrity::{introduced_transition, state_json};
use crate::{Snapshot, StoreError, StoreResult, validate_history};

pub const DB_NAME: &str = "review-episodes.sqlite";
pub const MARKER_NAME: &str = ".review-episode-offline-v1";
const MARKER_BYTES: &[u8] = b"review-episode-offline-v1\n";
const CURRENT_SQL: &str = "CREATE TABLE review_episode_current (identity_key TEXT PRIMARY KEY, revision TEXT NOT NULL UNIQUE CHECK(length(revision)=64), state_json TEXT NOT NULL) STRICT";
const HISTORY_SQL: &str = "CREATE TABLE review_episode_history (sequence INTEGER PRIMARY KEY AUTOINCREMENT, identity_key TEXT NOT NULL, revision TEXT NOT NULL UNIQUE CHECK(length(revision)=64), predecessor_revision TEXT, state_json TEXT NOT NULL) STRICT";

#[derive(Clone, Copy, Debug)]
pub struct StoreOptions {
    pub busy_timeout: Duration,
}
impl Default for StoreOptions {
    fn default() -> Self {
        Self {
            busy_timeout: Duration::from_millis(5_000),
        }
    }
}

pub struct EpisodeStore {
    pub(crate) conn: Option<Connection>,
    root: PathBuf,
}

pub enum WriteDisposition {
    Applied {
        state: Box<ReviewEpisodeState>,
        reply_json: String,
    },
    Replay {
        reply_json: String,
    },
}

impl EpisodeStore {
    pub fn open(root: &Path, options: StoreOptions) -> StoreResult<Self> {
        ensure_private_root(root)?;
        let marker = root.join(MARKER_NAME);
        verify_regular(&marker, false)?;
        if fs::read(&marker).map_err(io_err)? != MARKER_BYTES {
            return Err(StoreError::Path("offline root marker differs".into()));
        }
        let db = root.join(DB_NAME);
        let held = open_existing_nofollow(&db)?;
        verify_sidecars(root)?;
        let before = held.metadata().map_err(io_err)?;
        let conn = Connection::open_with_flags(
            &db,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(db_err)?;
        let after = fs::symlink_metadata(&db).map_err(io_err)?;
        if !same_file(&before, &after) {
            return Err(StoreError::Path(
                "database path changed during SQLite open".into(),
            ));
        }
        configure(&conn, options)?;
        verify_schema(&conn)?;
        verify_sidecars(root)?;
        drop(held);
        Ok(Self {
            conn: Some(conn),
            root: root.to_path_buf(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn read(&mut self, key: &str) -> StoreResult<Snapshot> {
        let tx = self
            .conn
            .as_mut()
            .ok_or(StoreError::OutcomeUnknown)?
            .transaction()
            .map_err(db_err)?;
        let snapshot = load_snapshot(&tx, key)?;
        if tx.commit().is_err() {
            self.conn.take();
            return Err(StoreError::OutcomeUnknown);
        }
        Ok(snapshot)
    }

    pub fn write<F>(
        &mut self,
        key: &str,
        observed: Option<&str>,
        max_reply: usize,
        apply: F,
    ) -> StoreResult<String>
    where
        F: FnOnce(Option<&ReviewEpisodeState>) -> StoreResult<WriteDisposition>,
    {
        let tx = self
            .conn
            .as_mut()
            .ok_or(StoreError::OutcomeUnknown)?
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_err)?;
        crate::test_checkpoint("after_begin_immediate");
        let prepared = (|| -> StoreResult<(String, bool)> {
            let snapshot = load_snapshot(&tx, key)?;
            if snapshot.current.as_ref().map(|s| s.revision().0.as_str()) != observed {
                return Err(StoreError::RevisionConflict);
            }
            let disposition = apply(snapshot.current.as_ref())?;
            match disposition {
                WriteDisposition::Replay { reply_json } => {
                    if reply_json.len() > max_reply {
                        return Err(StoreError::ResponseTooLarge);
                    }
                    Ok((reply_json, false))
                }
                WriteDisposition::Applied { state, reply_json } => {
                    if reply_json.len() > max_reply {
                        return Err(StoreError::ResponseTooLarge);
                    }
                    if state.identity().key().0 != key {
                        return Err(StoreError::Integrity(
                            "new state identity key differs".into(),
                        ));
                    }
                    introduced_transition(snapshot.current.as_ref(), &state)?;
                    let canonical = state_json(&state);
                    ReviewEpisodeState::from_stored_json(&canonical)
                        .map_err(|e| StoreError::Integrity(e.message))?;
                    let predecessor = snapshot.current.as_ref().map(|s| s.revision().0.as_str());
                    tx.execute("INSERT INTO review_episode_history(identity_key,revision,predecessor_revision,state_json) VALUES(?1,?2,?3,?4)", params![key, state.revision().0, predecessor, canonical]).map_err(db_err)?;
                    crate::test_checkpoint("after_history_insert");
                    let affected = if let Some(previous) = predecessor {
                        tx.execute("UPDATE review_episode_current SET revision=?1,state_json=?2 WHERE identity_key=?3 AND revision=?4", params![state.revision().0, canonical, key, previous]).map_err(db_err)?
                    } else {
                        tx.execute("INSERT INTO review_episode_current(identity_key,revision,state_json) VALUES(?1,?2,?3)", params![key, state.revision().0, canonical]).map_err(db_err)?
                    };
                    if affected != 1 {
                        return Err(StoreError::RevisionConflict);
                    }
                    crate::test_checkpoint("after_current_update");
                    Ok((reply_json, true))
                }
            }
        })();
        match prepared {
            Ok((reply_json, true)) => {
                crate::test_checkpoint("before_commit");
                if tx.commit().is_err() {
                    self.conn.take();
                    return Err(StoreError::OutcomeUnknown);
                }
                #[cfg(feature = "test-faults")]
                if std::env::var("REVIEW_EPISODE_FAULT_CUT").ok().as_deref()
                    == Some("after_commit_unknown")
                {
                    self.conn.take();
                    return Err(StoreError::OutcomeUnknown);
                }
                crate::test_checkpoint("after_commit");
                Ok(reply_json)
            }
            Ok((reply_json, false)) => {
                if tx.rollback().is_err() {
                    self.conn.take();
                    return Err(StoreError::OutcomeUnknown);
                }
                Ok(reply_json)
            }
            Err(error) => {
                if tx.rollback().is_err() {
                    self.conn.take();
                    return Err(StoreError::OutcomeUnknown);
                }
                Err(error)
            }
        }
    }

    pub fn validate_all(&mut self) -> StoreResult<(usize, usize)> {
        let tx = self
            .conn
            .as_mut()
            .ok_or(StoreError::OutcomeUnknown)?
            .transaction()
            .map_err(db_err)?;
        let integrity: String = tx
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .map_err(db_err)?;
        if integrity != "ok" {
            return Err(StoreError::Integrity(
                "SQLite integrity_check failed".into(),
            ));
        }
        let mut keys = Vec::new();
        {
            let mut statement = tx.prepare("SELECT identity_key FROM review_episode_current UNION SELECT identity_key FROM review_episode_history ORDER BY identity_key").map_err(db_err)?;
            let rows = statement
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(db_err)?;
            for row in rows {
                keys.push(row.map_err(db_err)?);
            }
        }
        let mut count = 0;
        for key in &keys {
            count += load_snapshot(&tx, key)?.history.len();
        }
        let max_sequence: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(sequence),0) FROM review_episode_history",
                [],
                |row| row.get(0),
            )
            .map_err(db_err)?;
        let highwater: i64 = tx
            .query_row(
                "SELECT seq FROM sqlite_sequence WHERE name='review_episode_history'",
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(db_err)?
            .unwrap_or(0);
        if highwater < max_sequence {
            return Err(StoreError::Integrity(
                "autoincrement high-water mark is below history".into(),
            ));
        }
        if tx.commit().is_err() {
            self.conn.take();
            return Err(StoreError::OutcomeUnknown);
        }
        Ok((keys.len(), count))
    }

    pub fn checkpoint(&self) -> StoreResult<()> {
        let result: (i64, i64, i64) = self
            .conn
            .as_ref()
            .ok_or(StoreError::OutcomeUnknown)?
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .map_err(db_err)?;
        if result.0 != 0 {
            return Err(StoreError::Busy);
        }
        Ok(())
    }
}

pub fn init_offline_root(root: &Path) -> StoreResult<()> {
    #[cfg(not(unix))]
    {
        let _ = root;
        return Err(StoreError::Path(
            "offline SQLite profile requires Unix".into(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        if !root.exists() {
            let mut builder = fs::DirBuilder::new();
            builder.mode(0o700);
            builder.create(root).map_err(io_err)?;
        }
        ensure_private_root(root)?;
        if fs::read_dir(root).map_err(io_err)?.next().is_some() {
            return Err(StoreError::Path("offline root must be empty".into()));
        }
        let marker = root.join(MARKER_NAME);
        let mut marker_file = create_nofollow(&marker)?;
        use std::io::Write;
        marker_file.write_all(MARKER_BYTES).map_err(io_err)?;
        marker_file.sync_all().map_err(io_err)?;
        let db = root.join(DB_NAME);
        let held = create_nofollow(&db)?;
        let conn = Connection::open_with_flags(
            &db,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(db_err)?;
        if !same_file(
            &held.metadata().map_err(io_err)?,
            &fs::symlink_metadata(&db).map_err(io_err)?,
        ) {
            return Err(StoreError::Path(
                "database path changed during initialization".into(),
            ));
        }
        configure(&conn, StoreOptions::default())?;
        let tx = conn.unchecked_transaction().map_err(db_err)?;
        tx.execute_batch(CURRENT_SQL).map_err(db_err)?;
        tx.execute_batch(HISTORY_SQL).map_err(db_err)?;
        tx.commit().map_err(|_| StoreError::OutcomeUnknown)?;
        verify_schema(&conn)?;
        drop(conn);
        File::open(root)
            .and_then(|file| file.sync_all())
            .map_err(io_err)?;
        Ok(())
    }
}

pub(crate) fn load_snapshot(conn: &Connection, key: &str) -> StoreResult<Snapshot> {
    let current: Option<(String, String)> = conn
        .query_row(
            "SELECT revision,state_json FROM review_episode_current WHERE identity_key=?1",
            [key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(db_err)?;
    let mut stmt = conn.prepare("SELECT sequence,identity_key,revision,predecessor_revision,state_json FROM review_episode_history WHERE identity_key=?1 ORDER BY sequence").map_err(db_err)?;
    let mapped = stmt
        .query_map([key], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .map_err(db_err)?;
    let mut rows = Vec::new();
    for row in mapped {
        rows.push(row.map_err(db_err)?);
    }
    validate_history(
        key,
        current.as_ref().map(|(r, j)| (r.as_str(), j.as_str())),
        rows,
    )
}

fn configure(conn: &Connection, options: StoreOptions) -> StoreResult<()> {
    conn.pragma_update(None, "trusted_schema", "OFF")
        .map_err(db_err)?;
    conn.busy_timeout(options.busy_timeout).map_err(db_err)?;
    let journal: String = conn
        .query_row("PRAGMA journal_mode=WAL", [], |r| r.get(0))
        .map_err(db_err)?;
    if !journal.eq_ignore_ascii_case("wal") {
        return Err(StoreError::Path("WAL journaling unavailable".into()));
    }
    conn.pragma_update(None, "synchronous", "FULL")
        .map_err(db_err)?;
    let synchronous: i64 = conn
        .query_row("PRAGMA synchronous", [], |r| r.get(0))
        .map_err(db_err)?;
    let trusted: i64 = conn
        .query_row("PRAGMA trusted_schema", [], |r| r.get(0))
        .map_err(db_err)?;
    let busy: i64 = conn
        .query_row("PRAGMA busy_timeout", [], |r| r.get(0))
        .map_err(db_err)?;
    if synchronous != 2 || trusted != 0 || busy != options.busy_timeout.as_millis() as i64 {
        return Err(StoreError::Path(
            "SQLite safety settings ineffective".into(),
        ));
    }
    Ok(())
}

fn normalized(sql: &str) -> String {
    sql.to_ascii_lowercase()
        .chars()
        .filter(|c| !c.is_ascii_whitespace() && *c != '"' && *c != '`')
        .collect::<String>()
        .replace("ifnotexists", "")
}

pub(crate) fn verify_schema(conn: &Connection) -> StoreResult<()> {
    let mut stmt = conn.prepare("SELECT type,name,sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_autoindex_%' ORDER BY name").map_err(db_err)?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(db_err)?;
    let mut objects = Vec::new();
    for row in rows {
        objects.push(row.map_err(db_err)?);
    }
    if objects.len() != 3 {
        return Err(StoreError::Schema("unexpected schema object count".into()));
    }
    for (kind, name, sql) in &objects {
        if kind != "table" {
            return Err(StoreError::Schema(
                "unexpected executable schema object".into(),
            ));
        }
        match name.as_str() {
            "review_episode_current"
                if normalized(sql.as_deref().unwrap_or("")) == normalized(CURRENT_SQL) => {}
            "review_episode_history"
                if normalized(sql.as_deref().unwrap_or("")) == normalized(HISTORY_SQL) => {}
            "sqlite_sequence" => (),
            _ => {
                return Err(StoreError::Schema(format!(
                    "unexpected table shape: {name}"
                )));
            }
        }
    }
    let strict_count: i64 = conn.query_row("SELECT COUNT(*) FROM pragma_table_list WHERE name IN ('review_episode_current','review_episode_history') AND strict=1", [], |r| r.get(0)).map_err(db_err)?;
    if strict_count != 2 {
        return Err(StoreError::Schema("episode tables must be STRICT".into()));
    }
    Ok(())
}

pub(crate) fn db_err(error: SqlError) -> StoreError {
    match &error {
        SqlError::SqliteFailure(inner, _)
            if matches!(
                inner.code,
                ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked
            ) =>
        {
            StoreError::Busy
        }
        _ => StoreError::Io(error.to_string()),
    }
}
pub(crate) fn io_err(error: std::io::Error) -> StoreError {
    StoreError::Io(error.to_string())
}

#[cfg(unix)]
fn uid() -> u32 {
    unsafe extern "C" {
        fn geteuid() -> u32;
    }
    unsafe { geteuid() }
}

#[cfg(unix)]
fn ensure_private_root(root: &Path) -> StoreResult<()> {
    use std::os::unix::fs::MetadataExt;
    let meta = fs::symlink_metadata(root).map_err(io_err)?;
    if !meta.is_dir()
        || meta.file_type().is_symlink()
        || meta.uid() != uid()
        || meta.mode() & 0o077 != 0
    {
        return Err(StoreError::Path(
            "offline root must be an owner-controlled private directory".into(),
        ));
    }
    Ok(())
}
#[cfg(not(unix))]
fn ensure_private_root(_: &Path) -> StoreResult<()> {
    Err(StoreError::Path(
        "offline SQLite profile requires Unix".into(),
    ))
}

#[cfg(unix)]
pub(crate) fn verify_regular(path: &Path, require_single_link: bool) -> StoreResult<()> {
    use std::os::unix::fs::MetadataExt;
    let m = fs::symlink_metadata(path).map_err(io_err)?;
    if !m.is_file()
        || m.file_type().is_symlink()
        || m.uid() != uid()
        || m.mode() & 0o077 != 0
        || (require_single_link && m.nlink() != 1)
    {
        return Err(StoreError::Path(format!(
            "unsafe regular file: {}",
            path.display()
        )));
    }
    Ok(())
}
#[cfg(not(unix))]
pub(crate) fn verify_regular(_: &Path, _: bool) -> StoreResult<()> {
    Err(StoreError::Path(
        "offline SQLite profile requires Unix".into(),
    ))
}

#[cfg(unix)]
fn same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    a.dev() == b.dev() && a.ino() == b.ino() && a.is_file() && b.is_file()
}
#[cfg(not(unix))]
fn same_file(_: &fs::Metadata, _: &fs::Metadata) -> bool {
    false
}

#[cfg(unix)]
pub(crate) fn create_nofollow(path: &Path) -> StoreResult<File> {
    use rustix::fs::{Mode, OFlags, openat};
    let parent = path
        .parent()
        .ok_or_else(|| StoreError::Path("missing parent".into()))?;
    let name = path
        .file_name()
        .ok_or_else(|| StoreError::Path("missing file name".into()))?;
    let dir = rustix::fs::open(
        parent,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW,
        Mode::empty(),
    )
    .map_err(|e| StoreError::Io(e.to_string()))?;
    let fd = openat(
        &dir,
        name,
        OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW,
        Mode::from_bits_retain(0o600),
    )
    .map_err(|e| StoreError::Io(e.to_string()))?;
    let file = File::from(fd);
    verify_regular(path, true)?;
    Ok(file)
}
#[cfg(not(unix))]
pub(crate) fn create_nofollow(_: &Path) -> StoreResult<File> {
    Err(StoreError::Path(
        "offline SQLite profile requires Unix".into(),
    ))
}

#[cfg(unix)]
pub(crate) fn open_existing_nofollow(path: &Path) -> StoreResult<File> {
    open_existing_with_access(path, true)
}
#[cfg(unix)]
pub(crate) fn open_existing_read_nofollow(path: &Path) -> StoreResult<File> {
    open_existing_with_access(path, false)
}
#[cfg(unix)]
fn open_existing_with_access(path: &Path, write: bool) -> StoreResult<File> {
    use rustix::fs::{Mode, OFlags, openat};
    let parent = path
        .parent()
        .ok_or_else(|| StoreError::Path("missing parent".into()))?;
    let name = path
        .file_name()
        .ok_or_else(|| StoreError::Path("missing name".into()))?;
    let dir = rustix::fs::open(
        parent,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW,
        Mode::empty(),
    )
    .map_err(|e| StoreError::Io(e.to_string()))?;
    let access = if write { OFlags::RDWR } else { OFlags::RDONLY };
    let fd = openat(&dir, name, access | OFlags::NOFOLLOW, Mode::empty())
        .map_err(|e| StoreError::Io(e.to_string()))?;
    let file = File::from(fd);
    verify_regular(path, true)?;
    if !same_file(
        &file.metadata().map_err(io_err)?,
        &fs::symlink_metadata(path).map_err(io_err)?,
    ) {
        return Err(StoreError::Path(
            "database file changed during opening".into(),
        ));
    }
    Ok(file)
}
#[cfg(not(unix))]
pub(crate) fn open_existing_nofollow(_: &Path) -> StoreResult<File> {
    Err(StoreError::Path(
        "offline SQLite profile requires Unix".into(),
    ))
}
#[cfg(not(unix))]
pub(crate) fn open_existing_read_nofollow(_: &Path) -> StoreResult<File> {
    Err(StoreError::Path(
        "offline SQLite profile requires Unix".into(),
    ))
}

pub(crate) fn verify_sidecars(root: &Path) -> StoreResult<()> {
    for name in [format!("{DB_NAME}-wal"), format!("{DB_NAME}-shm")] {
        let path = root.join(name);
        match fs::symlink_metadata(&path) {
            Ok(_) => verify_regular(&path, true)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(io_err(e)),
        }
    }
    Ok(())
}
