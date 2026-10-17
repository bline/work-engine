use std::fs::{self};
#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OpenFlags, params};

use crate::EvidenceReadError;
use crate::contract::{record_digest, require_sha, require_text, sha256};

const SCHEMA_VERSION: &str = "1";
pub const OWNER: &str = "review-execution-evidence";
pub const PROFILE: &str = "controlled-process-v1";
pub const VERIFIER: &str = "review-execution-evidence-process-v1";
pub const CLAUDE_PROFILE: &str = "claude-qualified-v1";
pub const CLAUDE_VERIFIER: &str = "review-execution-evidence-claude-v1";
pub const CLAUDE_TEST_PROFILE: &str = "claude-offline-peer-v1";
pub const CLAUDE_TEST_VERIFIER: &str = "review-execution-evidence-claude-offline-v1";

fn verifier_for(profile: &str) -> Result<&'static str, EvidenceReadError> {
    match profile {
        PROFILE => Ok(VERIFIER),
        CLAUDE_PROFILE => Ok(CLAUDE_VERIFIER),
        CLAUDE_TEST_PROFILE => Ok(CLAUDE_TEST_VERIFIER),
        _ => Err(EvidenceReadError::Unsupported("reader profile")),
    }
}

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
        verifier_for(&self.profile)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileId {
    dev: u64,
    ino: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnchorIds {
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

pub fn anchor_ids(root: &Path) -> Result<AnchorIds, EvidenceReadError> {
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

pub fn checked_root(path: &Path) -> Result<(PathBuf, PathBuf), EvidenceReadError> {
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

#[derive(Clone, Debug)]
pub struct Event {
    pub stage: String,
    pub payload: Vec<u8>,
    pub sha256: String,
}

pub fn read_events(conn: &Connection, attempt: &str) -> Result<Vec<Event>, EvidenceReadError> {
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

pub fn read_artifact(
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

pub fn open_reader(
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

pub fn check_marker(
    conn: &Connection,
    root: &Path,
    pin: &ReaderPin,
) -> Result<(), EvidenceReadError> {
    pin.validate()?;
    for (key, expected) in [
        ("schema_version", SCHEMA_VERSION),
        ("owner", OWNER),
        ("verifier", verifier_for(&pin.profile)?),
        ("profile", pin.profile.as_str()),
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
