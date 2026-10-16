use super::{Result, canonical_row_sha, sha, string, u64_field};
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde_json::{Value, json};
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::time::Duration;

// A read-only SQLite connection may initialize or modify WAL sidecars. Run this
// one-shot helper under a kernel-enforced no-filesystem-write Landlock domain
// before opening the DB. Unsupported kernels fail closed instead of inspecting
// a mutable scheduler source with unbounded side effects.
fn deny_filesystem_writes() -> Result<()> {
    const VERSION: libc::c_uint = 1;
    const WRITE_FILE: u64 = 1 << 1;
    const REMOVE_DIR: u64 = 1 << 4;
    const REMOVE_FILE: u64 = 1 << 5;
    const MAKE_CHAR: u64 = 1 << 6;
    const MAKE_DIR: u64 = 1 << 7;
    const MAKE_REG: u64 = 1 << 8;
    const MAKE_SOCK: u64 = 1 << 9;
    const MAKE_FIFO: u64 = 1 << 10;
    const MAKE_BLOCK: u64 = 1 << 11;
    const MAKE_SYM: u64 = 1 << 12;
    const REFER: u64 = 1 << 13;
    const TRUNCATE: u64 = 1 << 14;
    #[repr(C)]
    struct RulesetAttr {
        handled_access_fs: u64,
    }
    let abi = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            std::ptr::null::<u8>(),
            0,
            VERSION,
        )
    };
    if abi < 3 {
        return Err("kernel cannot enforce scheduler no-write read".into());
    }
    let attr = RulesetAttr {
        handled_access_fs: WRITE_FILE
            | REMOVE_DIR
            | REMOVE_FILE
            | MAKE_CHAR
            | MAKE_DIR
            | MAKE_REG
            | MAKE_SOCK
            | MAKE_FIFO
            | MAKE_BLOCK
            | MAKE_SYM
            | REFER
            | TRUNCATE,
    };
    let fd = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            &attr,
            std::mem::size_of::<RulesetAttr>(),
            0,
        )
    };
    if fd < 0 {
        return Err("cannot create scheduler no-write domain".into());
    }
    let no_new_privs = unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
    let restricted = if no_new_privs == 0 {
        unsafe { libc::syscall(libc::SYS_landlock_restrict_self, fd, 0) }
    } else {
        -1
    };
    unsafe {
        libc::close(fd as libc::c_int);
    }
    if restricted != 0 {
        return Err("cannot enforce scheduler no-write domain".into());
    }
    Ok(())
}

pub fn inspect(grant: &Value) -> Result<Value> {
    let descriptor = &grant["scheduler"];
    let path = Path::new(string(descriptor, "db_path")?);
    if !path.is_absolute() {
        return Err("scheduler source path is not absolute".into());
    }
    let before = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !before.is_file() || before.file_type().is_symlink() || before.len() > 4 * 1024 * 1024 {
        return Err("scheduler source file invalid".into());
    }
    deny_filesystem_writes()?;
    let db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| e.to_string())?;
    db.busy_timeout(Duration::from_secs(2))
        .map_err(|e| e.to_string())?;
    db.execute_batch("PRAGMA query_only=ON; BEGIN;")
        .map_err(|e| e.to_string())?;
    let integrity: String = db
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if integrity != "ok" {
        return Err("scheduler integrity check failed".into());
    }
    let revision: u64 = db
        .query_row("SELECT value FROM metadata WHERE key='revision'", [], |r| {
            r.get(0)
        })
        .map_err(|e| e.to_string())?;
    if revision != u64_field(descriptor, "store_revision")? {
        return Err("scheduler store revision changed".into());
    }
    let id = string(descriptor, "item_id")?;
    let row = db.query_row(
        "SELECT repository_id,logical_role,logical_agent_id,intent,due_at,on_due,approval_policy,catch_up_policy,result_policy,payload_json,status,created_at,updated_at,presented_at,result_ref FROM scheduled_items WHERE id=?1",
        params![id], |r| {
            Ok((r.get::<_,String>(0)?, r.get::<_,String>(1)?, r.get::<_,Option<String>>(2)?,
                r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,
                r.get::<_,String>(6)?,r.get::<_,String>(7)?,r.get::<_,String>(8)?,
                r.get::<_,String>(9)?,r.get::<_,String>(10)?,r.get::<_,String>(11)?,
                r.get::<_,String>(12)?,r.get::<_,Option<String>>(13)?,r.get::<_,Option<String>>(14)?))
        }).optional().map_err(|e| e.to_string())?.ok_or("scheduler item missing")?;
    let payload: Value = serde_json::from_str(&row.9).map_err(|e| e.to_string())?;
    let value = json!({"id":id,"repository_id":row.0,"logical_role":row.1,"logical_agent_id":row.2,
        "intent":row.3,"due_at":row.4,"on_due":row.5,"approval_policy":row.6,
        "catch_up_policy":row.7,"result_policy":row.8,"payload":payload,"status":row.10,
        "created_at":row.11,"updated_at":row.12,"presented_at":row.13,"result_ref":row.14});
    let digest = canonical_row_sha(&value)?;
    if digest != string(descriptor, "row_sha256")? {
        return Err("scheduler selected row changed".into());
    }
    if string(&value, "repository_id")? != string(descriptor, "repository_id")?
        || string(&value, "logical_role")? != string(descriptor, "logical_role")?
        || string(&value, "due_at")? != string(descriptor, "due_at")?
        || string(&value, "status")? != "acknowledged"
        || string(&value, "result_ref")? != string(descriptor, "result_ref")?
        || sha(string(&value, "intent")?.as_bytes()) != string(descriptor, "intent_sha256")?
    {
        return Err("scheduler row identity differs".into());
    }
    let intent = string(&value, "intent")?;
    let session_id = string(descriptor, "session_id")?;
    let episode_ref = string(descriptor, "episode_ref")?;
    if !intent.contains("candidate-4")
        || !intent.contains(session_id)
        || !intent.contains(episode_ref)
    {
        return Err("scheduler retry intent binding absent".into());
    }
    db.execute_batch("COMMIT;").map_err(|e| e.to_string())?;
    let after = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if before.dev() != after.dev() || before.ino() != after.ino() {
        return Err("scheduler source replaced during read".into());
    }
    Ok(
        json!({"source_owner":"role-scheduler","source_class":"current_retained_row",
        "item_id":id,"store_revision":revision,"row_sha256":digest,
        "repository_id":string(&value,"repository_id")?,"logical_role":string(&value,"logical_role")?,
        "candidate_attempt":4,"session_id":session_id,"episode_ref":episode_ref,
        "created_at":string(&value,"created_at")?,"due_at":string(&value,"due_at")?,
        "updated_at":string(&value,"updated_at")?,"presented_at":value["presented_at"],
        "status":string(&value,"status")?,"result_ref":string(&value,"result_ref")?,
        "source_limit":"One current acknowledged row; no mutation log, UUID causal edge, or presentation event"}),
    )
}
