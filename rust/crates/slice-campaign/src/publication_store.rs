use crate::{CampaignError, Result};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Serialize, de::DeserializeOwned};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::time::Duration;
use workspace_publication::{RecordRef, codec};

const MARKER: &str = ".campaign-publication-v1";
const DB: &str = "campaign-publication.sqlite";
const SCHEMA: &str = r#"
CREATE TABLE identity(singleton INTEGER PRIMARY KEY CHECK(singleton=1), root_id TEXT NOT NULL, campaign_root_id TEXT NOT NULL, installed_digest TEXT NOT NULL);
CREATE TABLE records(record_id TEXT PRIMARY KEY, kind TEXT NOT NULL, sha256 TEXT NOT NULL, payload BLOB NOT NULL);
CREATE TABLE packages(candidate_digest TEXT PRIMARY KEY, record_ref BLOB NOT NULL, campaign_revision TEXT NOT NULL);
CREATE TABLE eligibility(candidate_digest TEXT PRIMARY KEY, record_ref BLOB NOT NULL, active INTEGER NOT NULL, revision INTEGER NOT NULL);
CREATE TABLE offers(prepared_digest TEXT PRIMARY KEY, record_ref BLOB NOT NULL, state TEXT NOT NULL, revision INTEGER NOT NULL, decision_ref BLOB, consumption_ref BLOB, result_ref BLOB, cancellation_ref BLOB);
CREATE TABLE request_ids(request_id TEXT PRIMARY KEY, request_digest TEXT NOT NULL, record_ref BLOB NOT NULL);
"#;

pub(crate) struct PublicationStore {
    pub root_id: String,
    pub conn: Connection,
}
fn sql<T>(r: rusqlite::Result<T>) -> Result<T> {
    r.map_err(|e| CampaignError::Io(e.to_string()))
}
fn random_id() -> Result<String> {
    let mut b = [0u8; 16];
    File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut b))
        .map_err(|e| CampaignError::Io(e.to_string()))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}
impl PublicationStore {
    pub fn begin_write(&self) -> Result<Transaction<'_>> {
        sql(Transaction::new_unchecked(
            &self.conn,
            TransactionBehavior::Immediate,
        ))
    }
    pub fn initialize(root: &Path, campaign_root_id: &str, installed_digest: &str) -> Result<Self> {
        if !root.is_absolute() {
            return Err(CampaignError::Root("publication root not absolute".into()));
        }
        fs::create_dir(root).map_err(|e| CampaignError::Root(e.to_string()))?;
        fs::set_permissions(root, fs::Permissions::from_mode(0o700))
            .map_err(|e| CampaignError::Root(e.to_string()))?;
        let root_id = random_id()?;
        let marker = serde_json::json!({"rootId":root_id,"campaignRootId":campaign_root_id,"installedDigest":installed_digest});
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(root.join(MARKER))
            .map_err(|e| CampaignError::Root(e.to_string()))?;
        f.write_all(&codec::canonical(&marker).map_err(err)?)
            .and_then(|_| f.sync_all())
            .map_err(|e| CampaignError::Io(e.to_string()))?;
        let conn = sql(Connection::open(root.join(DB)))?;
        fs::set_permissions(root.join(DB), fs::Permissions::from_mode(0o600))
            .map_err(|e| CampaignError::Root(e.to_string()))?;
        configure(&conn)?;
        sql(conn.execute_batch(SCHEMA))?;
        sql(conn.execute(
            "INSERT INTO identity VALUES(1,?1,?2,?3)",
            params![root_id, campaign_root_id, installed_digest],
        ))?;
        Ok(Self { root_id, conn })
    }
    pub fn open(root: &Path, campaign_root_id: &str, installed_digest: &str) -> Result<Self> {
        let marker: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join(MARKER)).map_err(|e| CampaignError::Root(e.to_string()))?,
        )
        .map_err(|e| CampaignError::Integrity(e.to_string()))?;
        let root_id = marker["rootId"]
            .as_str()
            .ok_or_else(|| CampaignError::Integrity("publication root ID".into()))?
            .to_string();
        if marker["campaignRootId"] != campaign_root_id
            || marker["installedDigest"] != installed_digest
        {
            return Err(CampaignError::Conflict(
                "publication installation differs".into(),
            ));
        }
        let conn = sql(Connection::open(root.join(DB)))?;
        configure(&conn)?;
        let saved: (String, String, String) = sql(conn.query_row(
            "SELECT root_id,campaign_root_id,installed_digest FROM identity WHERE singleton=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ))?;
        if saved
            != (
                root_id.clone(),
                campaign_root_id.into(),
                installed_digest.into(),
            )
        {
            return Err(CampaignError::Integrity(
                "publication marker/DB differs".into(),
            ));
        }
        Ok(Self { root_id, conn })
    }
    pub fn put<T: Serialize>(&self, kind: &str, id: &str, value: &T) -> Result<RecordRef> {
        self.put_on(&self.conn, kind, id, value)
    }
    pub fn put_tx<T: Serialize>(
        &self,
        tx: &Transaction<'_>,
        kind: &str,
        id: &str,
        value: &T,
    ) -> Result<RecordRef> {
        self.put_on(tx, kind, id, value)
    }
    fn put_on<T: Serialize>(
        &self,
        conn: &Connection,
        kind: &str,
        id: &str,
        value: &T,
    ) -> Result<RecordRef> {
        let bytes = codec::canonical(value).map_err(err)?;
        let sha = codec::digest(kind, value).map_err(err)?;
        let old: Option<(String, String, Vec<u8>)> = sql(conn
            .query_row(
                "SELECT kind,sha256,payload FROM records WHERE record_id=?1",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional())?;
        if let Some((k, s, b)) = old {
            if k != kind || s != sha || b != bytes {
                return Err(CampaignError::Conflict(
                    "publication record ID collision".into(),
                ));
            }
        } else {
            sql(conn.execute(
                "INSERT INTO records VALUES(?1,?2,?3,?4)",
                params![id, kind, sha, bytes],
            ))?;
        }
        Ok(RecordRef {
            kind: kind.into(),
            schema_version: 1,
            root_id: self.root_id.clone(),
            record_id: id.into(),
            sha256: sha,
        })
    }
    pub fn get<T: DeserializeOwned + Serialize>(&self, r: &RecordRef) -> Result<T> {
        if r.root_id != self.root_id || r.schema_version != 1 {
            return Err(CampaignError::Conflict(
                "publication record owner/version".into(),
            ));
        }
        let (kind, sha, bytes): (String, String, Vec<u8>) = sql(self.conn.query_row(
            "SELECT kind,sha256,payload FROM records WHERE record_id=?1",
            [&r.record_id],
            |x| Ok((x.get(0)?, x.get(1)?, x.get(2)?)),
        ))?;
        if kind != r.kind || sha != r.sha256 {
            return Err(CampaignError::Integrity(
                "publication record reference".into(),
            ));
        }
        let value: T =
            serde_json::from_slice(&bytes).map_err(|e| CampaignError::Integrity(e.to_string()))?;
        if codec::canonical(&value).map_err(err)? != bytes
            || codec::digest(&kind, &value).map_err(err)? != sha
        {
            return Err(CampaignError::Integrity(
                "publication record content".into(),
            ));
        }
        Ok(value)
    }
    pub fn replay(&self, request_id: &str, request_digest: &str) -> Result<Option<RecordRef>> {
        self.replay_on(&self.conn, request_id, request_digest)
    }
    pub fn replay_tx(
        &self,
        tx: &Transaction<'_>,
        request_id: &str,
        request_digest: &str,
    ) -> Result<Option<RecordRef>> {
        self.replay_on(tx, request_id, request_digest)
    }
    fn replay_on(
        &self,
        conn: &Connection,
        request_id: &str,
        request_digest: &str,
    ) -> Result<Option<RecordRef>> {
        let old: Option<(String, Vec<u8>)> = sql(conn
            .query_row(
                "SELECT request_digest,record_ref FROM request_ids WHERE request_id=?1",
                [request_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional())?;
        match old {
            Some((digest, bytes)) if digest == request_digest => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|e| CampaignError::Integrity(e.to_string())),
            Some(_) => Err(CampaignError::Conflict(
                "request ID reused with different bytes".into(),
            )),
            None => Ok(None),
        }
    }
    pub fn remember_tx(
        &self,
        tx: &Transaction<'_>,
        request_id: &str,
        request_digest: &str,
        r: &RecordRef,
    ) -> Result<()> {
        self.remember_on(tx, request_id, request_digest, r)
    }
    fn remember_on(
        &self,
        conn: &Connection,
        request_id: &str,
        request_digest: &str,
        r: &RecordRef,
    ) -> Result<()> {
        sql(conn.execute(
            "INSERT INTO request_ids VALUES(?1,?2,?3)",
            params![
                request_id,
                request_digest,
                codec::canonical(r).map_err(err)?
            ],
        ))?;
        Ok(())
    }
}
fn err(e: workspace_publication::Error) -> CampaignError {
    CampaignError::Contract(e.to_string())
}
fn configure(conn: &Connection) -> Result<()> {
    sql(conn.busy_timeout(Duration::from_secs(30)))?;
    sql(conn
        .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;"))
}
