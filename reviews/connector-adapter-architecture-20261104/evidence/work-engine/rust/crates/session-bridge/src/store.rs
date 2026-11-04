use crate::wire::{Config, digest};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

pub type Result<T> = std::result::Result<T, &'static str>;
fn fail<T, E>(v: std::result::Result<T, E>) -> Result<T> {
    v.map_err(|_| "bridge_unavailable")
}
pub fn now() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp()
}
pub fn utc(s: &str) -> Result<i64> {
    OffsetDateTime::parse(s, &Rfc3339)
        .map(|t| t.unix_timestamp())
        .map_err(|_| "bridge_unavailable")
}
fn absolute(p: &str) -> Result<()> {
    if Path::new(p).is_absolute() {
        Ok(())
    } else {
        Err("bridge_unavailable")
    }
}
pub(crate) fn secure(p: &Path, directory: bool) -> Result<()> {
    let meta = fail(fs::symlink_metadata(p))?;
    if meta.file_type().is_symlink()
        || (directory && !meta.is_dir())
        || (!directory && !meta.is_file())
        || meta.uid() != unsafe { libc::geteuid() }
        || meta.mode() & 0o077 != 0
    {
        return Err("bridge_unavailable");
    }
    let mut parent = p.parent();
    if !directory {
        let m = fail(fs::symlink_metadata(parent.ok_or("bridge_unavailable")?))?;
        if m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0 {
            return Err("bridge_unavailable");
        }
    }
    while let Some(path) = parent {
        let m = fail(fs::symlink_metadata(path))?;
        if m.file_type().is_symlink() || !m.is_dir() || m.mode() & 0o022 != 0 {
            return Err("bridge_unavailable");
        }
        parent = path.parent();
    }
    Ok(())
}
pub fn new_private(p: &str) -> Result<File> {
    absolute(p)?;
    let parent = Path::new(p).parent().ok_or("bridge_unavailable")?;
    secure(parent, true)?;
    fail(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(p),
    )
}
pub fn config(path: &str) -> Result<Config> {
    absolute(path)?;
    secure(Path::new(path), false)?;
    let mut b = Vec::new();
    fail(File::open(path).and_then(|f| f.take(16 * 1024 + 1).read_to_end(&mut b)))?;
    if b.len() > 16 * 1024 {
        return Err("bridge_unavailable");
    }
    let c: Config = fail(serde_json::from_slice(&b))?;
    if c.schema_version != 1 || Uuid::parse_str(&c.store_id).is_err() {
        return Err("bridge_unavailable");
    }
    absolute(&c.database)?;
    absolute(&c.lock_dir)?;
    absolute(&c.native_socket)?;
    secure(Path::new(&c.database), false)?;
    secure(Path::new(&c.lock_dir), true)?;
    Ok(c)
}
pub fn initialize(config_path: &str, db: &str, lock_dir: &str, socket: &str) -> Result<Value> {
    absolute(db)?;
    absolute(lock_dir)?;
    absolute(socket)?;
    secure(Path::new(lock_dir), true)?;
    let mut config_file = new_private(config_path)?;
    let db_file = match new_private(db) {
        Ok(f) => f,
        Err(e) => {
            let _ = fs::remove_file(config_path);
            return Err(e);
        }
    };
    drop(db_file);
    let result = (|| {
        let conn = fail(Connection::open(db))?;
        fail(conn.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; CREATE TABLE meta (id TEXT NOT NULL, version INTEGER NOT NULL, lock_dir TEXT NOT NULL, native_socket TEXT NOT NULL); CREATE TABLE grants (grant_id TEXT PRIMARY KEY, token_hash TEXT NOT NULL UNIQUE, request_id TEXT NOT NULL UNIQUE, session_id TEXT NOT NULL, workspace TEXT NOT NULL, payload_hash TEXT NOT NULL, wire_hash TEXT NOT NULL, operation_hash TEXT NOT NULL, expires INTEGER NOT NULL, issuer_uid INTEGER NOT NULL, observed_at INTEGER NOT NULL, revoked INTEGER NOT NULL DEFAULT 0); CREATE TABLE operations (request_id TEXT PRIMARY KEY, operation_id TEXT NOT NULL UNIQUE, grant_id TEXT NOT NULL, session_id TEXT NOT NULL, workspace TEXT NOT NULL, payload_hash TEXT NOT NULL, wire_hash TEXT NOT NULL, operation_hash TEXT NOT NULL, wire TEXT NOT NULL, admitted INTEGER NOT NULL, state TEXT NOT NULL, baseline TEXT NOT NULL, ack TEXT, turn_id TEXT, native_status TEXT, reconciled INTEGER); CREATE TABLE occupancy (session_id TEXT PRIMARY KEY, request_id TEXT NOT NULL UNIQUE); CREATE TABLE transitions (seq INTEGER PRIMARY KEY, request_id TEXT NOT NULL, at INTEGER NOT NULL, state TEXT NOT NULL);"))?;
        let id = Uuid::new_v4().to_string();
        fail(conn.execute(
            "INSERT INTO meta VALUES (?1,1,?2,?3)",
            params![id, lock_dir, socket],
        ))?;
        let content = json!({"schema_version":1,"store_id":id,"database":db,"lock_dir":lock_dir,"native_socket":socket});
        fail(writeln!(config_file, "{}", content))?;
        fail(config_file.sync_all())?;
        Ok(content)
    })();
    if result.is_err() {
        let _ = fs::remove_file(config_path);
        let _ = fs::remove_file(db);
    }
    result
}
pub fn open(c: &Config) -> Result<Connection> {
    let conn = fail(Connection::open_with_flags(
        &c.database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE,
    ))?;
    fail(conn.execute_batch(
        "PRAGMA synchronous=FULL; PRAGMA busy_timeout=10000; PRAGMA foreign_keys=ON;",
    ))?;
    let meta: (String, i64, String, String) = fail(conn.query_row(
        "SELECT id,version,lock_dir,native_socket FROM meta",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    ))?;
    let count: i64 = fail(conn.query_row("SELECT count(*) FROM meta", [], |r| r.get(0)))?;
    if meta.0 != c.store_id
        || meta.1 != 1
        || meta.2 != c.lock_dir
        || meta.3 != c.native_socket
        || count != 1
    {
        return Err("bridge_unavailable");
    }
    let check: String = fail(conn.query_row("PRAGMA quick_check", [], |r| r.get(0)))?;
    if check != "ok" {
        return Err("bridge_unavailable");
    }
    Ok(conn)
}

pub fn save_wire(c: &Config, operation: &str, text: &str) -> Result<()> {
    if !crate::wire::valid_uuid(operation) {
        return Err("bridge_unavailable");
    }
    let path = Path::new(&c.lock_dir).join(format!("{operation}.input"));
    let mut f = fail(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path),
    )?;
    fail(f.write_all(text.as_bytes()))?;
    fail(f.sync_all())?;
    fail(File::open(&c.lock_dir).and_then(|d| d.sync_all()))?;
    Ok(())
}
pub fn load_wire(c: &Config, operation: &str, expected: &str) -> Result<String> {
    if !crate::wire::valid_uuid(operation) {
        return Err("bridge_unavailable");
    }
    let path = Path::new(&c.lock_dir).join(format!("{operation}.input"));
    secure(&path, false)?;
    let mut data = Vec::new();
    fail(File::open(path).and_then(|f| f.take(256 * 1024 + 1).read_to_end(&mut data)))?;
    if data.len() > 256 * 1024 || digest(&data) != expected {
        return Err("bridge_unavailable");
    }
    String::from_utf8(data).map_err(|_| "bridge_unavailable")
}
pub fn token_hash(token: &str) -> String {
    digest(format!("work-engine/session-bridge/token/v1\0{token}").as_bytes())
}
pub fn lock(c: &Config, session: &str) -> Result<File> {
    scoped_lock(c, session, "", true)
}
pub fn permission_lock(c: &Config, permission: &str) -> Result<File> {
    scoped_lock(c, permission, "permission-", false)
}
pub fn try_session_lock(c: &Config, session: &str) -> Result<File> {
    scoped_lock(c, session, "", false)
}
fn scoped_lock(c: &Config, session: &str, prefix: &str, wait: bool) -> Result<File> {
    if !crate::wire::valid_uuid(session) {
        return Err("bridge_unavailable");
    }
    let p = Path::new(&c.lock_dir).join(format!("{prefix}{session}.lock"));
    let f = fail(
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(&p),
    )?;
    secure(&p, false)?;
    if wait {
        fail(f.lock())?;
    } else {
        f.try_lock().map_err(|_| "session_busy")?;
    }
    Ok(f)
}
#[allow(clippy::too_many_arguments)]
pub fn issue(
    conn: &mut Connection,
    request: &str,
    session: &str,
    workspace: &str,
    payload_hash: &str,
    wire_hash: &str,
    expires: i64,
    credential_out: &str,
) -> Result<Value> {
    if expires <= now() {
        return Err("expired");
    }
    let mut random = [0u8; 32];
    fail(getrandom::fill(&mut random))?;
    let credential = format!("webridge_{}", URL_SAFE_NO_PAD.encode(random));
    let id = Uuid::new_v4().to_string();
    let operation_hash =
        crate::wire::operation_digest(request, session, workspace, payload_hash, wire_hash);
    let mut f = new_private(credential_out)?;
    let result = (|| {
        fail(writeln!(f, "{credential}"))?;
        fail(f.sync_all())?;
        let tx = fail(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
        tx.execute("INSERT INTO grants (grant_id,token_hash,request_id,session_id,workspace,payload_hash,wire_hash,operation_hash,expires,issuer_uid,observed_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)", params![id,token_hash(&credential),request,session,workspace,payload_hash,wire_hash,operation_hash,expires,unsafe { libc::geteuid() },now()]).map_err(|_| "payload_conflict")?;
        fail(tx.commit())?;
        Ok(
            json!({"schema_version":1,"grant_id":id,"request_id":request,"session_id":session,"workspace":workspace,"payload_sha256":payload_hash,"wire_sha256":wire_hash,"operation_sha256":operation_hash,"envelope_version":"runtime-envelope-v1","admit_until":expires,"message_source":"runtime","capability_scope":"message_admission_only","revocation":"blocks subsequent reads and submissions; does not cancel admitted work","dedicated_session_required":true,"observed_at":now(),"observed_target":"loaded, idle, canonical workspace, complete bounded stable history","issuer_uid":unsafe { libc::geteuid() },"runtime_policy":"unavailable"}),
        )
    })();
    if result.is_err() {
        let _ = fs::remove_file(credential_out);
    }
    result
}
pub fn revoke(conn: &mut Connection, id: &str) -> Result<Value> {
    let tx = fail(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
    let n = fail(tx.execute("UPDATE grants SET revoked=1 WHERE grant_id=?1", [id]))?;
    fail(tx.commit())?;
    if n != 1 {
        return Err("bridge_unavailable");
    }
    Ok(json!({"schema_version":1,"grant_id":id,"revoked":true,"cancels_admitted_work":false}))
}
#[derive(Clone)]
pub struct Grant {
    pub id: String,
    pub request: String,
    pub session: String,
    pub workspace: String,
    pub payload_hash: String,
    pub wire_hash: String,
    pub operation_hash: String,
}
pub fn authorize(conn: &Connection, request: &str, token: &str) -> Result<Grant> {
    let row = fail(conn.query_row("SELECT grant_id,request_id,session_id,workspace,payload_hash,wire_hash,operation_hash,expires,revoked FROM grants WHERE token_hash=?1", [token_hash(token)], |r| Ok((Grant{id:r.get(0)?,request:r.get(1)?,session:r.get(2)?,workspace:r.get(3)?,payload_hash:r.get(4)?,wire_hash:r.get(5)?,operation_hash:r.get(6)?},r.get::<_,i64>(8)?))).optional())?;
    match row {
        Some((g, 0)) if g.request == request => {
            if !crate::wire::valid_uuid(&g.id)
                || !crate::wire::valid_uuid(&g.request)
                || !crate::wire::valid_uuid(&g.session)
                || crate::wire::operation_digest(
                    &g.request,
                    &g.session,
                    &g.workspace,
                    &g.payload_hash,
                    &g.wire_hash,
                ) != g.operation_hash
            {
                return Err("bridge_unavailable");
            }
            Ok(g)
        }
        _ => Err("unauthorized"),
    }
}
pub fn transition(conn: &Connection, request: &str, state: &str) -> Result<()> {
    fail(conn.execute(
        "INSERT INTO transitions(request_id,at,state) VALUES (?1,?2,?3)",
        params![request, now(), state],
    ))?;
    Ok(())
}
pub fn project(conn: &Connection, request: &str) -> Result<Option<Value>> {
    fail(conn.query_row("SELECT operation_id,grant_id,session_id,workspace,payload_hash,wire_hash,admitted,state,turn_id,native_status,reconciled,operation_hash FROM operations WHERE request_id=?1",[request],|r| Ok(json!({"schema_version":1,"operation_id":r.get::<_,String>(0)?,"request_id":request,"grant_id":r.get::<_,String>(1)?,"session_id":r.get::<_,String>(2)?,"workspace":r.get::<_,String>(3)?,"payload_sha256":r.get::<_,String>(4)?,"wire_sha256":r.get::<_,String>(5)?,"admitted_at":r.get::<_,i64>(6)?,"entry_state":r.get::<_,String>(7)?,"native_turn_id":r.get::<_,Option<String>>(8)?,"native_status":r.get::<_,Option<String>>(9)?,"last_reconciliation_at":r.get::<_,Option<i64>>(10)?,"operation_sha256":r.get::<_,String>(11)?,"envelope_version":"runtime-envelope-v1","profile":"dedicated-loaded-session-v1","message_source":"runtime","capability_scope":"message_admission_only","limitations":["dedicated session required; no native idle fence","process closure unavailable"]}))).optional())
}
