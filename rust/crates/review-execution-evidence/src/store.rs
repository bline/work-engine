use std::fs::{self, File, OpenOptions};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, params};

use crate::contract::{record_digest, require_sha, require_text, sha256};
use crate::{EvidenceReadError, ROOT_PAYLOAD_MAX_BYTES};

const SCHEMA_VERSION: &str = "1";
pub(crate) const OWNER: &str = "review-execution-evidence";
pub(crate) const PROFILE: &str = "controlled-process-v1";
pub(crate) const VERIFIER: &str = "review-execution-evidence-process-v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReaderPin {
    pub root_id: String,
    pub configuration_sha256: String,
    pub executable_sha256: String,
    pub source_sha256: String,
    pub profile: String,
}

impl ReaderPin {
    pub fn validate(&self) -> Result<(), EvidenceReadError> {
        require_text(&self.root_id)?;
        for digest in [
            &self.configuration_sha256,
            &self.executable_sha256,
            &self.source_sha256,
        ] {
            require_sha(digest)?;
        }
        if self.profile != PROFILE {
            return Err(EvidenceReadError::Unsupported("reader profile"));
        }
        Ok(())
    }
}

pub(crate) struct Store {
    pub root: PathBuf,
    pub db: PathBuf,
    pub conn: Connection,
    _root_fence: File,
    _writer_fence: File,
    anchors: AnchorIds,
    pin: ReaderPin,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FileId {
    dev: u64,
    ino: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AnchorIds {
    root: FileId,
    db: FileId,
    fence: FileId,
}

#[cfg(unix)]
fn file_id(meta: &fs::Metadata) -> FileId {
    FileId {
        dev: meta.dev(),
        ino: meta.ino(),
    }
}

#[cfg(not(unix))]
compile_error!("controlled execution evidence currently qualifies Unix file identity only");

fn regular_id(path: &Path, label: &'static str) -> Result<FileId, EvidenceReadError> {
    let meta = fs::symlink_metadata(path).map_err(|_| EvidenceReadError::Unresolved(label))?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err(EvidenceReadError::Conflicting(
            "unsafe evidence file or link",
        ));
    }
    Ok(file_id(&meta))
}

pub(crate) fn anchor_ids(root: &Path) -> Result<AnchorIds, EvidenceReadError> {
    let (root, db) = checked_root(root)?;
    let root_meta = fs::symlink_metadata(&root)
        .map_err(|_| EvidenceReadError::Unresolved("root identity inaccessible"))?;
    Ok(AnchorIds {
        root: file_id(&root_meta),
        db: regular_id(&db, "database identity inaccessible")?,
        fence: regular_id(
            &root.join("writer.lock"),
            "writer fence identity inaccessible",
        )?,
    })
}

fn verify_anchor(root: &Path, expected: AnchorIds) -> Result<(), EvidenceReadError> {
    if anchor_ids(root)? != expected {
        return Err(EvidenceReadError::Conflicting(
            "evidence root/database/fence identity changed",
        ));
    }
    Ok(())
}

pub(crate) fn checked_root(path: &Path) -> Result<(PathBuf, PathBuf), EvidenceReadError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(EvidenceReadError::Unsupported(
            "root must be absolute without parent traversal",
        ));
    }
    let meta = fs::symlink_metadata(path)
        .map_err(|_| EvidenceReadError::Unresolved("evidence root inaccessible"))?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(EvidenceReadError::Conflicting("unsafe evidence root"));
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| EvidenceReadError::Unresolved("root canonicalization failed"))?;
    if canonical != path {
        return Err(EvidenceReadError::Conflicting("evidence root path changed"));
    }
    let db = path.join("evidence.sqlite");
    let db_meta = fs::symlink_metadata(&db)
        .map_err(|_| EvidenceReadError::Unresolved("evidence database inaccessible"))?;
    if !db_meta.is_file() || db_meta.file_type().is_symlink() {
        return Err(EvidenceReadError::Conflicting("unsafe evidence database"));
    }
    for sidecar in ["evidence.sqlite-wal", "evidence.sqlite-shm"] {
        let sidecar_path = path.join(sidecar);
        match fs::symlink_metadata(&sidecar_path) {
            Ok(meta) if !meta.is_file() || meta.file_type().is_symlink() => {
                return Err(EvidenceReadError::Conflicting("unsafe SQLite sidecar"));
            }
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(EvidenceReadError::Unresolved("SQLite sidecar inaccessible")),
        }
    }
    Ok((canonical, db))
}

fn fence(root: &Path, create_new: bool) -> Result<(File, File), EvidenceReadError> {
    // The directory inode is the primary live-writer fence. Replacing the
    // named lock file cannot give a second writer a different live fence for
    // the same root directory.
    let root_file =
        File::open(root).map_err(|_| EvidenceReadError::Unresolved("root fence inaccessible"))?;
    let root_id = file_id(
        &root_file
            .metadata()
            .map_err(|_| EvidenceReadError::Unresolved("held root identity inaccessible"))?,
    );
    let path_id = file_id(
        &fs::symlink_metadata(root)
            .map_err(|_| EvidenceReadError::Unresolved("root identity inaccessible"))?,
    );
    if root_id != path_id {
        return Err(EvidenceReadError::Conflicting("root changed before fence"));
    }
    root_file
        .try_lock()
        .map_err(|_| EvidenceReadError::Unresolved("another root writer is active"))?;
    if root_id
        != file_id(
            &fs::symlink_metadata(root)
                .map_err(|_| EvidenceReadError::Unresolved("root identity inaccessible"))?,
        )
    {
        return Err(EvidenceReadError::Conflicting("root changed after fence"));
    }
    let path = root.join("writer.lock");
    if !create_new {
        regular_id(&path, "writer fence inaccessible")?;
    }
    let file = OpenOptions::new()
        .create_new(create_new)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|_| EvidenceReadError::Unresolved("writer fence inaccessible"))?;
    let opened = file
        .metadata()
        .map_err(|_| EvidenceReadError::Unresolved("opened writer fence inaccessible"))?;
    if file_id(&opened) != regular_id(&path, "writer fence inaccessible")? {
        return Err(EvidenceReadError::Conflicting("writer fence path changed"));
    }
    file.try_lock()
        .map_err(|_| EvidenceReadError::Unresolved("another evidence writer is active"))?;
    if file_id(&opened) != regular_id(&path, "writer fence inaccessible")? {
        return Err(EvidenceReadError::Conflicting(
            "writer fence replaced after lock",
        ));
    }
    Ok((root_file, file))
}

fn configure_writer(conn: &Connection) -> Result<(), EvidenceReadError> {
    conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA busy_timeout=5000;")
        .map_err(|_| EvidenceReadError::Unresolved("writer database configuration failed"))
}

impl Store {
    pub fn reserve_terminal_capacity(&self) -> Result<(), EvidenceReadError> {
        self.check_live_identity()?;
        let artifacts: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(length(bytes)),0) FROM artifacts",
                [],
                |r| r.get(0),
            )
            .map_err(|_| EvidenceReadError::Unresolved("artifact capacity query failed"))?;
        let events: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(length(payload)),0) FROM events",
                [],
                |r| r.get(0),
            )
            .map_err(|_| EvidenceReadError::Unresolved("event capacity query failed"))?;
        if artifacts < 0
            || events < 0
            || (artifacts as usize)
                .saturating_add(events as usize)
                .saturating_add(crate::TERMINAL_MAX_BYTES)
                > ROOT_PAYLOAD_MAX_BYTES
        {
            return Err(EvidenceReadError::Capacity(
                "terminal reserve exceeds root limit",
            ));
        }
        Ok(())
    }

    pub fn create(root: &Path, pin: &ReaderPin) -> Result<Self, EvidenceReadError> {
        pin.validate()?;
        if !root.is_absolute() || root.exists() {
            return Err(EvidenceReadError::Conflicting(
                "evidence root already exists or is relative",
            ));
        }
        let parent = root
            .parent()
            .ok_or(EvidenceReadError::Unsupported("root parent missing"))?;
        if parent
            .canonicalize()
            .map_err(|_| EvidenceReadError::Unresolved("root parent inaccessible"))?
            != parent
        {
            return Err(EvidenceReadError::Conflicting(
                "root parent is not canonical",
            ));
        }
        fs::create_dir(root).map_err(|_| EvidenceReadError::Unresolved("root creation failed"))?;
        let (root_fence, lock) = fence(root, true)?;
        let db = root.join("evidence.sqlite");
        let conn = Connection::open_with_flags(
            &db,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )
        .map_err(|_| EvidenceReadError::Unresolved("database creation failed"))?;
        configure_writer(&conn)?;
        conn.execute_batch(include_str!("../migrations/0001_evidence.sql"))
            .map_err(|_| EvidenceReadError::Unresolved("schema creation failed"))?;
        let anchors = anchor_ids(root)?;
        for (key, value) in [
            ("schema_version", SCHEMA_VERSION),
            ("owner", OWNER),
            ("verifier", VERIFIER),
            ("profile", PROFILE),
            ("root_id", pin.root_id.as_str()),
            ("configuration_sha256", pin.configuration_sha256.as_str()),
            ("executable_sha256", pin.executable_sha256.as_str()),
            ("source_sha256", pin.source_sha256.as_str()),
            (
                "root_path",
                root.to_str()
                    .ok_or(EvidenceReadError::Unsupported("root path not UTF-8"))?,
            ),
        ] {
            conn.execute(
                "INSERT INTO metadata(key,value) VALUES(?1,?2)",
                params![key, value],
            )
            .map_err(|_| EvidenceReadError::Unresolved("marker write failed"))?;
        }
        write_anchor_marker(&conn, anchors)?;
        let (root, db) = checked_root(root)?;
        verify_anchor(&root, anchors)?;
        Ok(Self {
            root,
            db,
            conn,
            _root_fence: root_fence,
            _writer_fence: lock,
            anchors,
            pin: pin.clone(),
        })
    }

    pub fn resume(root: &Path, pin: &ReaderPin) -> Result<Self, EvidenceReadError> {
        // Read-only preflight precedes creating or locking any writer resource
        // and precedes PRAGMAs that can change journal mode or sidecars.
        let (root, db, preflight) = open_reader(root, pin)?;
        let anchors = anchor_ids(&root)?;
        drop(preflight);
        let (root_fence, lock) = fence(&root, false)?;
        verify_anchor(&root, anchors)?;
        let (_, _, recheck) = open_reader(&root, pin)?;
        drop(recheck);
        let conn = Connection::open_with_flags(&db, OpenFlags::SQLITE_OPEN_READ_WRITE)
            .map_err(|_| EvidenceReadError::Unresolved("database open failed"))?;
        check_marker(&conn, &root, pin)?;
        verify_anchor(&root, anchors)?;
        configure_writer(&conn)?;
        verify_anchor(&root, anchors)?;
        Ok(Self {
            root,
            db,
            conn,
            _root_fence: root_fence,
            _writer_fence: lock,
            anchors,
            pin: pin.clone(),
        })
    }

    pub fn append(
        &mut self,
        attempt: &str,
        stage: &str,
        payload: &[u8],
        artifacts: &[(&str, &[u8])],
    ) -> Result<String, EvidenceReadError> {
        self.check_live_identity()?;
        let (_, current_db) = checked_root(&self.root)?;
        if current_db != self.db {
            return Err(EvidenceReadError::Conflicting(
                "writer database path changed",
            ));
        }
        require_text(attempt)?;
        require_text(stage)?;
        if payload.len() > crate::TERMINAL_MAX_BYTES {
            return Err(EvidenceReadError::Capacity("event payload exceeds limit"));
        }
        let existing = self.events(attempt)?;
        if existing.iter().any(|event| event.stage == stage) {
            return Err(EvidenceReadError::Conflicting("stage already recorded"));
        }
        let prior = existing.last().map_or("", |event| event.sha256.as_str());
        let digest = record_digest(&[
            attempt.as_bytes(),
            stage.as_bytes(),
            prior.as_bytes(),
            payload,
        ]);
        let tx = self
            .conn
            .transaction()
            .map_err(|_| EvidenceReadError::Unresolved("event transaction failed"))?;
        let artifact_total: i64 = tx
            .query_row(
                "SELECT COALESCE(SUM(length(bytes)),0) FROM artifacts",
                [],
                |r| r.get(0),
            )
            .map_err(|_| EvidenceReadError::Unresolved("root capacity read failed"))?;
        let event_total: i64 = tx
            .query_row(
                "SELECT COALESCE(SUM(length(payload)),0) FROM events",
                [],
                |r| r.get(0),
            )
            .map_err(|_| EvidenceReadError::Unresolved("event capacity read failed"))?;
        let incoming: usize = artifacts
            .iter()
            .map(|(_, bytes)| bytes.len())
            .sum::<usize>()
            + payload.len();
        if artifact_total < 0
            || event_total < 0
            || (artifact_total as usize)
                .saturating_add(event_total as usize)
                .saturating_add(incoming)
                > ROOT_PAYLOAD_MAX_BYTES
        {
            return Err(EvidenceReadError::Capacity("root payload limit"));
        }
        for (kind, bytes) in artifacts {
            tx.execute(
                "INSERT INTO artifacts(attempt_id,kind,bytes,sha256) VALUES(?1,?2,?3,?4)",
                params![attempt, kind, bytes, sha256(bytes)],
            )
            .map_err(|_| {
                EvidenceReadError::Conflicting("artifact already recorded or inaccessible")
            })?;
        }
        tx.execute("INSERT INTO events(attempt_id,ordinal,stage,predecessor_sha256,payload,sha256) VALUES(?1,?2,?3,?4,?5,?6)",
            params![attempt, existing.len() as i64, stage, prior, payload, digest])
            .map_err(|_| EvidenceReadError::Conflicting("event already recorded or inaccessible"))?;
        tx.commit()
            .map_err(|_| EvidenceReadError::Unresolved("event commit uncertain"))?;
        self.check_live_identity()?;
        Ok(digest)
    }

    pub fn events(&self, attempt: &str) -> Result<Vec<Event>, EvidenceReadError> {
        self.check_live_identity()?;
        let events = read_events(&self.conn, attempt)?;
        self.check_live_identity()?;
        Ok(events)
    }

    fn check_live_identity(&self) -> Result<(), EvidenceReadError> {
        verify_anchor(&self.root, self.anchors)?;
        if file_id(
            &self
                ._root_fence
                .metadata()
                .map_err(|_| EvidenceReadError::Unresolved("held root fence inaccessible"))?,
        ) != self.anchors.root
        {
            return Err(EvidenceReadError::Conflicting("held root fence changed"));
        }
        if file_id(
            &self
                ._writer_fence
                .metadata()
                .map_err(|_| EvidenceReadError::Unresolved("held fence inaccessible"))?,
        ) != self.anchors.fence
        {
            return Err(EvidenceReadError::Conflicting("held writer fence changed"));
        }
        check_marker(&self.conn, &self.root, &self.pin)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Event {
    pub stage: String,
    pub payload: Vec<u8>,
    pub sha256: String,
}

pub(crate) fn read_events(
    conn: &Connection,
    attempt: &str,
) -> Result<Vec<Event>, EvidenceReadError> {
    let mut stmt = conn.prepare("SELECT ordinal,stage,predecessor_sha256,length(payload),payload,sha256 FROM events WHERE attempt_id=?1 ORDER BY ordinal")
        .map_err(|_| EvidenceReadError::Unresolved("event query failed"))?;
    let mut rows = stmt
        .query([attempt])
        .map_err(|_| EvidenceReadError::Unresolved("event query failed"))?;
    let mut events: Vec<Event> = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|_| EvidenceReadError::Unresolved("event read failed"))?
    {
        let ordinal: i64 = row
            .get(0)
            .map_err(|_| EvidenceReadError::Conflicting("event ordinal invalid"))?;
        let stage: String = row
            .get(1)
            .map_err(|_| EvidenceReadError::Conflicting("event stage invalid"))?;
        let prior: String = row
            .get(2)
            .map_err(|_| EvidenceReadError::Conflicting("event predecessor invalid"))?;
        let length: i64 = row
            .get(3)
            .map_err(|_| EvidenceReadError::Conflicting("event length invalid"))?;
        if length < 0 || length as usize > crate::TERMINAL_MAX_BYTES {
            return Err(EvidenceReadError::Capacity("event payload exceeds limit"));
        }
        let payload: Vec<u8> = row
            .get(4)
            .map_err(|_| EvidenceReadError::Conflicting("event payload invalid"))?;
        let digest: String = row
            .get(5)
            .map_err(|_| EvidenceReadError::Conflicting("event digest invalid"))?;
        if payload.len() != length as usize
            || ordinal != events.len() as i64
            || prior != events.last().map_or("", |event| event.sha256.as_str())
            || digest
                != record_digest(&[
                    attempt.as_bytes(),
                    stage.as_bytes(),
                    prior.as_bytes(),
                    &payload,
                ])
        {
            return Err(EvidenceReadError::Conflicting("event chain invalid"));
        }
        events.push(Event {
            stage,
            payload,
            sha256: digest,
        });
        if events.len() > 8 {
            return Err(EvidenceReadError::Capacity("too many attempt events"));
        }
    }
    Ok(events)
}

pub(crate) fn read_artifact(
    conn: &Connection,
    attempt: &str,
    kind: &str,
    limit: usize,
) -> Result<Vec<u8>, EvidenceReadError> {
    let (size, digest): (i64, String) = conn
        .query_row(
            "SELECT length(bytes),sha256 FROM artifacts WHERE attempt_id=?1 AND kind=?2",
            params![attempt, kind],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|err| {
            if matches!(err, rusqlite::Error::QueryReturnedNoRows) {
                EvidenceReadError::Unresolved("required artifact missing")
            } else {
                EvidenceReadError::Unresolved("artifact metadata inaccessible")
            }
        })?;
    if size < 0 || size as usize > limit {
        return Err(EvidenceReadError::Capacity("artifact exceeds limit"));
    }
    let bytes: Vec<u8> = conn
        .query_row(
            "SELECT bytes FROM artifacts WHERE attempt_id=?1 AND kind=?2",
            params![attempt, kind],
            |r| r.get(0),
        )
        .map_err(|_| EvidenceReadError::Unresolved("artifact inaccessible"))?;
    if bytes.len() != size as usize || sha256(&bytes) != digest {
        return Err(EvidenceReadError::Conflicting(
            "artifact bytes/digest mismatch",
        ));
    }
    Ok(bytes)
}

pub(crate) fn open_reader(
    root: &Path,
    pin: &ReaderPin,
) -> Result<(PathBuf, PathBuf, Connection), EvidenceReadError> {
    let (root, db) = checked_root(root)?;
    let conn = Connection::open_with_flags(&db, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| EvidenceReadError::Unresolved("reader database open failed"))?;
    conn.busy_timeout(Duration::from_millis(crate::MAX_DATABASE_BUSY_MILLIS))
        .map_err(|_| EvidenceReadError::Unresolved("reader busy policy failed"))?;
    conn.execute_batch("PRAGMA query_only=ON; PRAGMA trusted_schema=OFF;")
        .map_err(|_| EvidenceReadError::Unresolved("reader safety configuration failed"))?;
    check_marker(&conn, &root, pin)?;
    anchor_ids(&root)?;
    Ok((root, db, conn))
}

fn marker_value(conn: &Connection, key: &str) -> Result<String, EvidenceReadError> {
    conn.query_row("SELECT value FROM metadata WHERE key=?1", [key], |r| {
        r.get(0)
    })
    .map_err(|_| EvidenceReadError::Conflicting("root marker missing"))
}

fn write_anchor_marker(conn: &Connection, anchors: AnchorIds) -> Result<(), EvidenceReadError> {
    for (key, value) in [
        ("root_dev", anchors.root.dev),
        ("root_ino", anchors.root.ino),
        ("db_dev", anchors.db.dev),
        ("db_ino", anchors.db.ino),
        ("fence_dev", anchors.fence.dev),
        ("fence_ino", anchors.fence.ino),
    ] {
        conn.execute(
            "INSERT INTO metadata(key,value) VALUES(?1,?2)",
            params![key, value.to_string()],
        )
        .map_err(|_| EvidenceReadError::Unresolved("anchor marker write failed"))?;
    }
    Ok(())
}

pub(crate) fn check_marker(
    conn: &Connection,
    root: &Path,
    pin: &ReaderPin,
) -> Result<(), EvidenceReadError> {
    pin.validate()?;
    for (key, expected) in [
        ("schema_version", SCHEMA_VERSION),
        ("owner", OWNER),
        ("verifier", VERIFIER),
        ("profile", PROFILE),
        ("root_id", pin.root_id.as_str()),
        ("configuration_sha256", pin.configuration_sha256.as_str()),
        ("executable_sha256", pin.executable_sha256.as_str()),
        ("source_sha256", pin.source_sha256.as_str()),
        (
            "root_path",
            root.to_str()
                .ok_or(EvidenceReadError::Unsupported("root path not UTF-8"))?,
        ),
    ] {
        let actual = marker_value(conn, key)?;
        if actual != expected {
            return Err(EvidenceReadError::Conflicting("root marker mismatch"));
        }
    }
    let anchors = anchor_ids(root)?;
    for (key, expected) in [
        ("root_dev", anchors.root.dev),
        ("root_ino", anchors.root.ino),
        ("db_dev", anchors.db.dev),
        ("db_ino", anchors.db.ino),
        ("fence_dev", anchors.fence.dev),
        ("fence_ino", anchors.fence.ino),
    ] {
        if marker_value(conn, key)? != expected.to_string() {
            return Err(EvidenceReadError::Conflicting(
                "anchored file identity mismatch",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::contract::record_digest;

    #[test]
    fn prepared_event_codec_v1_golden_vector() {
        assert_eq!(
            record_digest(&[b"attempt-1", b"prepared", b"", b"{\"schema_version\":1}"]),
            "6aafe1580525d76b44f7f97aceb54d064b1630255eef69d763629c679c176fed"
        );
    }
}
