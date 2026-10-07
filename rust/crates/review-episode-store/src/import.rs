//! Byte-preserving import of a closed, explicitly supplied SQLite copy.
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, OpenFlags, params};
use sha2::{Digest, Sha256};

use crate::sqlite::{
    DB_NAME, db_err, init_offline_root, load_snapshot, open_existing_read_nofollow, verify_regular,
    verify_schema,
};
use crate::{EpisodeStore, StoreError, StoreOptions, StoreResult};

#[derive(Clone, Debug)]
pub struct CopyManifest {
    pub source: PathBuf,
    pub sha256: String,
    pub provenance: String,
}
#[derive(Clone, Debug)]
pub struct ImportReport {
    pub schema_profile: &'static str,
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    pub declared_provenance: String,
    pub source_sha256: String,
    pub destination_sha256: String,
    pub episodes: usize,
    pub rows: usize,
    pub history_highwater: Option<i64>,
    pub user_version: i64,
    pub byte_equal: bool,
}

type CurrentRow = (String, String, String);
type HistoryRow = (i64, String, String, Option<String>, String);

struct Dump {
    current: Vec<CurrentRow>,
    history: Vec<HistoryRow>,
    highwater: Option<i64>,
    user_version: i64,
}

pub fn import_closed_copy(
    manifest: &CopyManifest,
    destination: &Path,
) -> StoreResult<ImportReport> {
    if manifest.provenance.trim().is_empty() {
        return Err(StoreError::Path(
            "copy provenance declaration is empty".into(),
        ));
    }
    if manifest.sha256.len() != 64
        || !manifest
            .sha256
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(StoreError::Path("copy manifest SHA-256 is invalid".into()));
    }
    if destination.exists() {
        return Err(StoreError::Path("destination already exists".into()));
    }
    verify_regular(&manifest.source, true)?;
    reject_sidecars(&manifest.source)?;
    let source_canonical = manifest
        .source
        .canonicalize()
        .map_err(crate::sqlite::io_err)?;
    let parent = destination
        .parent()
        .ok_or_else(|| StoreError::Path("destination parent missing".into()))?;
    if !parent.is_dir() {
        return Err(StoreError::Path("destination parent missing".into()));
    }
    let destination_abs = parent.canonicalize().map_err(crate::sqlite::io_err)?.join(
        destination
            .file_name()
            .ok_or_else(|| StoreError::Path("destination name missing".into()))?,
    );
    if source_canonical == destination_abs || source_canonical == destination_abs.join(DB_NAME) {
        return Err(StoreError::Path("source/destination alias".into()));
    }
    let source_before = sha_file(&manifest.source)?;
    if source_before != manifest.sha256 {
        return Err(StoreError::Integrity(
            "copy digest differs from manifest".into(),
        ));
    }
    let held = open_existing_read_nofollow(&manifest.source)?;
    let source = Connection::open_with_flags(
        &manifest.source,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(db_err)?;
    if !same_held(&held, &manifest.source)? {
        return Err(StoreError::Path(
            "copy source changed during SQLite open".into(),
        ));
    }
    source
        .pragma_update(None, "trusted_schema", "OFF")
        .map_err(db_err)?;
    source
        .pragma_update(None, "query_only", "ON")
        .map_err(db_err)?;
    verify_schema(&source)?;
    let dump = dump_and_validate(&source)?;
    drop(source);
    crate::test_checkpoint("import_after_source_validation");
    if sha_file(&manifest.source)? != source_before {
        return Err(StoreError::Integrity(
            "copy changed during validation".into(),
        ));
    }
    reject_sidecars(&manifest.source)?;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| StoreError::Io(e.to_string()))?
        .as_nanos();
    let stage = parent.join(format!(
        ".review-episode-import-{}-{nonce}",
        std::process::id()
    ));
    let outcome = (|| {
        init_offline_root(&stage)?;
        let mut target = EpisodeStore::open(&stage, StoreOptions::default())?;
        {
            let tx = target
                .conn
                .as_mut()
                .ok_or(StoreError::OutcomeUnknown)?
                .transaction()
                .map_err(db_err)?;
            for (sequence, key, revision, predecessor, json) in &dump.history {
                tx.execute("INSERT INTO review_episode_history(sequence,identity_key,revision,predecessor_revision,state_json) VALUES(?1,?2,?3,?4,?5)",params![sequence,key,revision,predecessor,json]).map_err(db_err)?;
            }
            for (key, revision, json) in &dump.current {
                tx.execute("INSERT INTO review_episode_current(identity_key,revision,state_json) VALUES(?1,?2,?3)",params![key,revision,json]).map_err(db_err)?;
            }
            tx.execute(
                "DELETE FROM sqlite_sequence WHERE name='review_episode_history'",
                [],
            )
            .map_err(db_err)?;
            if let Some(highwater) = dump.highwater {
                tx.execute(
                    "INSERT INTO sqlite_sequence(name,seq) VALUES('review_episode_history',?1)",
                    [highwater],
                )
                .map_err(db_err)?;
            }
            tx.pragma_update(None, "user_version", dump.user_version)
                .map_err(db_err)?;
            crate::test_checkpoint("import_before_commit");
            tx.commit().map_err(|_| StoreError::OutcomeUnknown)?;
            crate::test_checkpoint("import_after_commit");
        }
        target.validate_all()?;
        target.checkpoint()?;
        drop(target);
        let mut reopened = EpisodeStore::open(&stage, StoreOptions::default())?;
        let (episodes, rows) = reopened.validate_all()?;
        let after = dump_and_validate(reopened.conn.as_ref().ok_or(StoreError::OutcomeUnknown)?)?;
        if dump.current != after.current
            || dump.history != after.history
            || dump.highwater != after.highwater
            || dump.user_version != after.user_version
        {
            return Err(StoreError::Integrity(
                "import row/counter byte comparison failed".into(),
            ));
        }
        reopened.checkpoint()?;
        drop(reopened);
        if sha_file(&manifest.source)? != source_before {
            return Err(StoreError::Integrity("copy changed during import".into()));
        }
        reject_sidecars(&manifest.source)?;
        if destination.exists() {
            return Err(StoreError::Path(
                "destination appeared during import".into(),
            ));
        }
        File::open(&stage)
            .and_then(|dir| dir.sync_all())
            .map_err(crate::sqlite::io_err)?;
        crate::test_checkpoint("import_before_publish");
        fs::rename(&stage, destination).map_err(crate::sqlite::io_err)?;
        crate::test_checkpoint("import_after_publish");
        File::open(parent)
            .and_then(|dir| dir.sync_all())
            .map_err(|_| StoreError::OutcomeUnknown)?;
        let destination_sha256 = sha_file(&destination.join(DB_NAME))?;
        Ok(ImportReport {
            schema_profile: "review-episode-strict-v1",
            source_path: manifest.source.clone(),
            destination_path: destination.to_path_buf(),
            declared_provenance: manifest.provenance.clone(),
            source_sha256: source_before,
            destination_sha256,
            episodes,
            rows,
            history_highwater: dump.highwater,
            user_version: dump.user_version,
            byte_equal: true,
        })
    })();
    if outcome.is_err() && stage.exists() {
        let _ = fs::remove_dir_all(&stage);
    }
    outcome
}

/// Validate an already published destination after a lost import response.
/// No row import or destination replacement is attempted by this route.
pub fn reconcile_closed_copy(
    manifest: &CopyManifest,
    destination: &Path,
) -> StoreResult<ImportReport> {
    if !destination.exists() {
        return Err(StoreError::Path("import destination is absent".into()));
    }
    if manifest.provenance.trim().is_empty() {
        return Err(StoreError::Path(
            "copy provenance declaration is empty".into(),
        ));
    }
    verify_regular(&manifest.source, true)?;
    if manifest
        .source
        .canonicalize()
        .map_err(crate::sqlite::io_err)?
        == destination
            .join(DB_NAME)
            .canonicalize()
            .map_err(crate::sqlite::io_err)?
    {
        return Err(StoreError::Path("source/destination alias".into()));
    }
    reject_sidecars(&manifest.source)?;
    let source_sha256 = sha_file(&manifest.source)?;
    if source_sha256 != manifest.sha256 {
        return Err(StoreError::Integrity(
            "copy digest differs from manifest".into(),
        ));
    }
    let held = open_existing_read_nofollow(&manifest.source)?;
    let source = Connection::open_with_flags(
        &manifest.source,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(db_err)?;
    if !same_held(&held, &manifest.source)? {
        return Err(StoreError::Path(
            "copy changed during reconciliation open".into(),
        ));
    }
    source
        .pragma_update(None, "trusted_schema", "OFF")
        .map_err(db_err)?;
    source
        .pragma_update(None, "query_only", "ON")
        .map_err(db_err)?;
    let original = dump_and_validate(&source)?;
    drop(source);
    let mut published = EpisodeStore::open(destination, StoreOptions::default())?;
    let (episodes, rows) = published.validate_all()?;
    let copied = dump_and_validate(published.conn.as_ref().ok_or(StoreError::OutcomeUnknown)?)?;
    if original.current != copied.current
        || original.history != copied.history
        || original.highwater != copied.highwater
        || original.user_version != copied.user_version
    {
        return Err(StoreError::Integrity(
            "published destination differs from supplied copy".into(),
        ));
    }
    published.checkpoint()?;
    drop(published);
    if sha_file(&manifest.source)? != source_sha256 {
        return Err(StoreError::Integrity(
            "copy changed during reconciliation".into(),
        ));
    }
    reject_sidecars(&manifest.source)?;
    let destination_sha256 = sha_file(&destination.join(DB_NAME))?;
    Ok(ImportReport {
        schema_profile: "review-episode-strict-v1",
        source_path: manifest.source.clone(),
        destination_path: destination.to_path_buf(),
        declared_provenance: manifest.provenance.clone(),
        source_sha256,
        destination_sha256,
        episodes,
        rows,
        history_highwater: copied.highwater,
        user_version: copied.user_version,
        byte_equal: true,
    })
}

fn dump_and_validate(conn: &Connection) -> StoreResult<Dump> {
    verify_schema(conn)?;
    let check: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(db_err)?;
    if check != "ok" {
        return Err(StoreError::Integrity(
            "SQLite integrity_check failed".into(),
        ));
    }
    let mut current: Vec<CurrentRow> = Vec::new();
    {
        let mut stmt=conn.prepare("SELECT identity_key,revision,state_json FROM review_episode_current ORDER BY identity_key").map_err(db_err)?;
        let mapped = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .map_err(db_err)?;
        for row in mapped {
            current.push(row.map_err(db_err)?);
        }
    }
    let mut history: Vec<HistoryRow> = Vec::new();
    {
        let mut stmt=conn.prepare("SELECT sequence,identity_key,revision,predecessor_revision,state_json FROM review_episode_history ORDER BY sequence").map_err(db_err)?;
        let mapped = stmt
            .query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })
            .map_err(db_err)?;
        for row in mapped {
            history.push(row.map_err(db_err)?);
        }
    }
    let mut keys: Vec<_> = current
        .iter()
        .map(|x| x.0.clone())
        .chain(history.iter().map(|x| x.1.clone()))
        .collect();
    keys.sort();
    keys.dedup();
    for key in keys {
        load_snapshot(conn, &key)?;
    }
    let mut highwater_stmt = conn
        .prepare("SELECT seq FROM sqlite_sequence WHERE name='review_episode_history'")
        .map_err(db_err)?;
    let highwater: Vec<i64> = highwater_stmt
        .query_map([], |r| r.get(0))
        .map_err(db_err)?
        .collect::<Result<_, _>>()
        .map_err(db_err)?;
    if highwater.len() > 1 {
        return Err(StoreError::Integrity(
            "copy has duplicate history high-water rows".into(),
        ));
    }
    let highwater = highwater.into_iter().next();
    let max_seq = history.last().map_or(0, |r| r.0);
    if highwater.unwrap_or(0) < max_seq {
        return Err(StoreError::Integrity(
            "copy high-water mark is invalid".into(),
        ));
    }
    let user_version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(db_err)?;
    Ok(Dump {
        current,
        history,
        highwater,
        user_version,
    })
}

fn sha_file(path: &Path) -> StoreResult<String> {
    let mut file = open_existing_read_nofollow(path)?;
    let mut digest = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let count = file.read(&mut buf).map_err(crate::sqlite::io_err)?;
        if count == 0 {
            break;
        }
        digest.update(&buf[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}
fn reject_sidecars(source: &Path) -> StoreResult<()> {
    for suffix in ["-wal", "-shm"] {
        let path = PathBuf::from(format!("{}{suffix}", source.display()));
        if fs::symlink_metadata(&path).is_ok() {
            return Err(StoreError::Path(
                "closed copy still has SQLite sidecar".into(),
            ));
        }
    }
    Ok(())
}
#[cfg(unix)]
fn same_held(held: &File, path: &Path) -> StoreResult<bool> {
    use std::os::unix::fs::MetadataExt;
    let a = held.metadata().map_err(crate::sqlite::io_err)?;
    let b = fs::symlink_metadata(path).map_err(crate::sqlite::io_err)?;
    Ok(a.dev() == b.dev() && a.ino() == b.ino())
}
#[cfg(not(unix))]
fn same_held(_: &File, _: &Path) -> StoreResult<bool> {
    Ok(false)
}
