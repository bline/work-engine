//! Local-only admission and receipt owner for the session-events bridge.
mod native;
mod provision;
mod store;
mod wire;

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde_json::{Value, json};
use std::{
    env, fs,
    io::{self, Read, Write},
};
use store::{Grant, Result};
use uuid::Uuid;
use wire::{Config, Rpc};

fn fail<T, E>(r: std::result::Result<T, E>) -> Result<T> {
    r.map_err(|_| "bridge_unavailable")
}
fn canonical(path: &str) -> Result<String> {
    let p = fail(fs::canonicalize(path))?;
    if !p.is_dir() {
        return Err("target_mismatch");
    }
    p.to_str().map(str::to_owned).ok_or("target_mismatch")
}
fn read_input(path: &str) -> Result<String> {
    let m = fail(fs::symlink_metadata(path))?;
    if !m.is_file() || m.file_type().is_symlink() || m.len() > wire::LIMIT as u64 {
        return Err("bridge_unavailable");
    }
    let mut b = Vec::new();
    fail(fs::File::open(path).and_then(|f| f.take(wire::LIMIT as u64 + 1).read_to_end(&mut b)))?;
    if b.len() > wire::LIMIT {
        return Err("bridge_unavailable");
    }
    String::from_utf8(b).map_err(|_| "bridge_unavailable")
}
fn cli(args: Vec<String>) -> Result<Value> {
    if args.len() < 3 || args[0] != "--config" {
        return Err("bridge_unavailable");
    }
    let path = &args[1];
    let command = &args[2..];
    if command.len() == 8
        && command[0] == "store"
        && command[1] == "init"
        && command[2] == "--database"
        && command[4] == "--lock-dir"
        && command[6] == "--native-socket"
    {
        return store::initialize(path, &command[3], &command[5], &command[7]);
    }
    let c = store::config(path)?;
    let mut conn = store::open(&c)?;
    if command == ["provisioning", "owner", "serve"] {
        return provision::serve(&c, &mut conn);
    }
    if command == ["provisioning", "init"] {
        return provision::initialize(&mut conn);
    }
    if command.len() == 6
        && command[0] == "provisioning"
        && command[1] == "issue"
        && command[2] == "--policy-file"
        && command[4] == "--credential-out"
    {
        return provision::issue(&mut conn, &command[3], &command[5]);
    }
    if command.len() == 4
        && command[0] == "provisioning"
        && command[1] == "revoke"
        && command[2] == "--permission-id"
    {
        let _lock = store::permission_lock(&c, &command[3])?;
        return provision::revoke(&mut conn, &command[3]);
    }
    if command == ["rpc"] {
        let mut bytes = Vec::new();
        fail(io::stdin().take(256 * 1024 + 1).read_to_end(&mut bytes))?;
        if bytes.len() > 256 * 1024 {
            return Err("bridge_unavailable");
        }
        let mut value: Value = serde_json::from_slice(&bytes).map_err(|_| "bridge_unavailable")?;
        if value.get("provisioning_credential").is_some() {
            let object = value.as_object_mut().ok_or("unauthorized")?;
            if object.remove("schema_version").and_then(|v| v.as_u64()) != Some(1)
                || object.contains_key("grant_credential")
            {
                return Err("unauthorized");
            }
            let request: provision::OwnerRpc =
                serde_json::from_value(value).map_err(|_| "unauthorized")?;
            let result = provision::send(&c, &request);
            if result.as_ref().is_err_and(|e| *e == "bridge_unavailable")
                && request.method == "read_session_target"
                && request.idempotency_key.is_none()
                && request.grant_handle.is_none()
                && request.request_id.is_none()
                && request.session_id.is_none()
                && request.workspace.is_none()
                && request.input_text.is_none()
            {
                let permission =
                    provision::authenticate(&conn, &request.provisioning_credential, false)?;
                let mut value = provision::projection(&conn, &permission.id)?;
                value["owner_health"] = json!("unavailable");
                return Ok(value);
            }
            return result;
        }
        let req: Rpc = serde_json::from_value(value).map_err(|_| "bridge_unavailable")?;
        return rpc(&c, &mut conn, req);
    }
    if command.len() == 4
        && command[0] == "grant"
        && command[1] == "revoke"
        && command[2] == "--grant-id"
    {
        return store::revoke(&mut conn, &command[3]);
    }
    if command.len() == 4
        && command[0] == "operation"
        && command[1] == "inspect"
        && command[2] == "--request-id"
    {
        return store::project(&conn, &command[3])?.ok_or("bridge_unavailable");
    }
    if command.len() == 10
        && command[0] == "grant"
        && command[1] == "preview"
        && command[2] == "--request-id"
        && command[4] == "--session-id"
        && command[6] == "--workspace"
        && command[8] == "--input-file"
    {
        let workspace = canonical(&command[7])?;
        let payload = read_input(&command[9])?;
        let (text, ph, wh) = wire::wire(&command[3], &command[5], &workspace, &payload)
            .map_err(|_| "bridge_unavailable")?;
        return Ok(
            json!({"schema_version":1,"wire_input":text,"payload_sha256":ph,"wire_sha256":wh,"workspace":workspace,"message_source":"runtime","capability_scope":"message_admission_only"}),
        );
    }
    if command.len() == 17
        && command[0] == "grant"
        && command[1] == "issue"
        && command[2] == "--request-id"
        && command[4] == "--session-id"
        && command[6] == "--workspace"
        && command[8] == "--input-file"
        && command[10] == "--admit-until"
        && command[12] == "--credential-out"
        && command[14] == "--dedicated-session"
        && command[15] == "--reviewed-wire-sha256"
    {
        let workspace = canonical(&command[7])?;
        let payload = read_input(&command[9])?;
        let (_, ph, wh) = wire::wire(&command[3], &command[5], &workspace, &payload)
            .map_err(|_| "bridge_unavailable")?;
        if command[16] != wh {
            return Err("payload_conflict");
        }
        let mut target =
            native::Native::connect(&c.native_socket).map_err(|_| "history_unavailable")?;
        target.preflight(&command[5], &workspace)?;
        return store::issue(
            &mut conn,
            &command[3],
            &command[5],
            &workspace,
            &ph,
            &wh,
            store::utc(&command[11])?,
            &command[13],
        );
    }
    Err("bridge_unavailable")
}
fn rpc(c: &Config, conn: &mut Connection, req: Rpc) -> Result<Value> {
    if req.schema_version != 1 || !wire::valid_uuid(&req.request_id) {
        return Err("unauthorized");
    }
    let g = store::authorize(conn, &req.request_id, &req.grant_credential)?;
    match req.method.as_str() {
        "submit_session_request" => {
            let (Some(session), Some(workspace), Some(input)) =
                (req.session_id, req.workspace, req.input_text)
            else {
                return Err("bridge_unavailable");
            };
            if session != g.session || workspace != g.workspace {
                return Err("target_mismatch");
            }
            let (text, ph, wh) = wire::wire(&req.request_id, &session, &workspace, &input)
                .map_err(|_| "payload_conflict")?;
            if ph != g.payload_hash
                || wh != g.wire_hash
                || wire::operation_digest(&g.request, &g.session, &g.workspace, &ph, &wh)
                    != g.operation_hash
            {
                return Err("payload_conflict");
            }
            submit(c, conn, &g, &text)
        }
        "read_session_operation" | "read_session_result" => {
            if req.session_id.is_some() || req.workspace.is_some() || req.input_text.is_some() {
                return Err("bridge_unavailable");
            }
            reconcile(c, conn, &g)
        }
        _ => Err("bridge_unavailable"),
    }
}
fn checked_projection(conn: &Connection, g: &Grant) -> Result<Option<Value>> {
    let Some(value) = store::project(conn, &g.request)? else {
        return Ok(None);
    };
    for (key, expected) in [
        ("grant_id", &g.id),
        ("request_id", &g.request),
        ("session_id", &g.session),
        ("workspace", &g.workspace),
        ("payload_sha256", &g.payload_hash),
        ("wire_sha256", &g.wire_hash),
        ("operation_sha256", &g.operation_hash),
    ] {
        if value.get(key).and_then(Value::as_str) != Some(expected) {
            return Err("bridge_unavailable");
        }
    }
    if !value
        .get("operation_id")
        .and_then(Value::as_str)
        .is_some_and(wire::valid_uuid)
    {
        return Err("bridge_unavailable");
    }
    let state = value
        .get("entry_state")
        .and_then(Value::as_str)
        .ok_or("bridge_unavailable")?;
    let occupied = [
        "admitted",
        "admitted_no_attempt",
        "entry_uncertain",
        "entered",
        "identity_conflict",
    ]
    .contains(&state);
    if !occupied && !["refused_before_send", "terminal"].contains(&state) {
        return Err("bridge_unavailable");
    }
    let holder: Option<String> = fail(
        conn.query_row(
            "SELECT request_id FROM occupancy WHERE session_id=?1",
            [&g.session],
            |r| r.get(0),
        )
        .optional(),
    )?;
    if occupied != (holder.as_deref() == Some(&g.request)) {
        return Err("bridge_unavailable");
    }
    if state == "entered" || state == "terminal" {
        if !value
            .get("native_turn_id")
            .and_then(Value::as_str)
            .is_some_and(wire::valid_uuid)
        {
            return Err("bridge_unavailable");
        }
        let status = value
            .get("native_status")
            .and_then(Value::as_str)
            .ok_or("bridge_unavailable")?;
        if (state == "entered" && status != "inProgress")
            || (state == "terminal" && !["completed", "failed", "interrupted"].contains(&status))
        {
            return Err("bridge_unavailable");
        }
    }
    Ok(Some(value))
}
fn submit(c: &Config, conn: &mut Connection, g: &Grant, text: &str) -> Result<Value> {
    submit_with_native(c, conn, g, text, None, None, None)
}
fn submit_with_native(
    c: &Config,
    conn: &mut Connection,
    g: &Grant,
    text: &str,
    held: Option<&mut native::Native>,
    admission_deadline: Option<std::time::Instant>,
    profile: Option<&provision::Policy>,
) -> Result<Value> {
    let _lock = if admission_deadline.is_some() {
        store::try_session_lock(c, &g.session)?
    } else {
        store::lock(c, &g.session)?
    };
    // Authorization is rechecked in the same write transaction as admission below.
    let revoked: i64 = fail(conn.query_row(
        "SELECT revoked FROM grants WHERE grant_id=?1",
        [&g.id],
        |r| r.get(0),
    ))?;
    if revoked != 0 {
        return Err("unauthorized");
    }
    if let Some(mut v) = checked_projection(conn, g)? {
        if v.get("entry_state").and_then(Value::as_str) == Some("admitted") {
            let tx = fail(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
            fail(tx.execute(
                "UPDATE operations SET state='admitted_no_attempt' WHERE request_id=?1",
                [&g.request],
            ))?;
            store::transition(&tx, &g.request, "admitted_no_attempt")?;
            fail(tx.commit())?;
            v = checked_projection(conn, g)?.ok_or("bridge_unavailable")?;
        }
        return Ok(v);
    }
    if canonical(&g.workspace)? != g.workspace {
        return Err("target_mismatch");
    }
    let mut temporary = None;
    if held.is_none() {
        temporary =
            Some(native::Native::connect(&c.native_socket).map_err(|_| "history_unavailable")?);
    }
    let native = held.or(temporary.as_mut()).ok_or("history_unavailable")?;
    let baseline = native.preflight(&g.session, &g.workspace)?;
    let baseline_ids: Vec<Value> = baseline.iter().map(|turn| json!({"id":turn["id"],"sha256":wire::digest(serde_json::to_string(turn).unwrap_or_default().as_bytes())})).collect();
    let baseline_json = fail(serde_json::to_string(&baseline_ids))?;
    let operation = Uuid::new_v4().to_string();
    let tx = fail(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
    let row: Option<(i64, i64)> = fail(
        tx.query_row(
            "SELECT expires,revoked FROM grants WHERE grant_id=?1",
            [&g.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional(),
    )?;
    let Some((expires, revoked)) = row else {
        return Err("unauthorized");
    };
    if revoked != 0 {
        return Err("unauthorized");
    }
    if expires <= store::now() {
        return Err("expired");
    }
    tx.execute(
        "INSERT INTO occupancy VALUES (?1,?2)",
        params![g.session, g.request],
    )
    .map_err(|_| "session_busy")?;
    store::save_wire(c, &operation, text)?;
    tx.execute("INSERT INTO operations(request_id,operation_id,grant_id,session_id,workspace,payload_hash,wire_hash,operation_hash,wire,admitted,state,baseline) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,'admitted',?11)",params![g.request,operation,g.id,g.session,g.workspace,g.payload_hash,g.wire_hash,g.operation_hash,operation,store::now(),baseline_json]).map_err(|_| "payload_conflict")?;
    store::transition(&tx, &g.request, "admitted")?;
    fail(tx.commit())?;
    let resumed = if let Some(profile) = profile {
        admission_deadline.is_some_and(|deadline| {
            native
                .call_until("thread/resume", json!({"threadId":g.session}), deadline)
                .is_ok_and(|v| provision::target_ack(&v, profile, Some(&g.session)).is_ok())
        })
    } else {
        native.resume(&g.session, &g.workspace).is_ok()
    };
    let ready = resumed
        && native
            .preflight(&g.session, &g.workspace)
            .is_ok_and(|history| history == baseline)
        && profile.is_none_or(|p| provision::recheck_at_entry(native, p, &g.session).is_ok());
    if !ready || canonical(&g.workspace).ok().as_deref() != Some(&g.workspace) {
        let tx = fail(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
        fail(tx.execute(
            "UPDATE operations SET state='refused_before_send' WHERE request_id=?1",
            [&g.request],
        ))?;
        fail(tx.execute("DELETE FROM occupancy WHERE request_id=?1", [&g.request]))?;
        store::transition(&tx, &g.request, "refused_before_send")?;
        fail(tx.commit())?;
        return checked_projection(conn, g)?.ok_or("bridge_unavailable");
    }
    let tx = fail(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
    if admission_deadline.is_some() {
        let (expires, revoked): (i64, i64) = fail(tx.query_row(
            "SELECT expires,revoked FROM grants WHERE grant_id=?1",
            [&g.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ))?;
        if revoked != 0
            || expires <= store::now()
            || admission_deadline.is_some_and(|d| std::time::Instant::now() >= d)
        {
            fail(tx.execute(
                "UPDATE operations SET state='refused_before_send' WHERE request_id=?1",
                [&g.request],
            ))?;
            fail(tx.execute("DELETE FROM occupancy WHERE request_id=?1", [&g.request]))?;
            store::transition(&tx, &g.request, "refused_before_send")?;
            fail(tx.commit())?;
            return checked_projection(conn, g)?.ok_or("bridge_unavailable");
        }
    }
    fail(tx.execute(
        "UPDATE operations SET state='entry_uncertain' WHERE request_id=?1",
        [&g.request],
    ))?;
    store::transition(&tx, &g.request, "entry_uncertain")?;
    fail(tx.commit())?;
    #[cfg(all(feature = "test-hooks", debug_assertions))]
    if env::var_os("SESSION_BRIDGE_TEST_STOP_BEFORE_WRITE").is_some() {
        // The process-level crash test stops exactly after durable intent; the
        // normal binary contains no test stop or externally controlled hook.
        unsafe { libc::raise(libc::SIGSTOP) };
    }
    let started = match admission_deadline {
        Some(deadline) => native.start_until(&g.session, &operation, text, deadline),
        None => native.start(&g.session, &operation, text),
    };
    if let Ok(ack) = started {
        #[cfg(all(feature = "test-hooks", debug_assertions))]
        if env::var_os("SESSION_BRIDGE_TEST_STOP_BEFORE_ACK_PERSIST").is_some() {
            unsafe { libc::raise(libc::SIGSTOP) };
        }
        let tx = fail(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
        fail(tx.execute(
            "UPDATE operations SET ack=?1 WHERE request_id=?2",
            params![ack, g.request],
        ))?;
        store::transition(&tx, &g.request, "acknowledged")?;
        fail(tx.commit())?;
    }
    checked_projection(conn, g)?.ok_or("bridge_unavailable")
}
fn reconcile(c: &Config, conn: &mut Connection, g: &Grant) -> Result<Value> {
    reconcile_with_native(c, conn, g, None, false)
}
fn reconcile_with_native(
    c: &Config,
    conn: &mut Connection,
    g: &Grant,
    held: Option<&mut native::Native>,
    bounded_lock: bool,
) -> Result<Value> {
    let _lock = if bounded_lock {
        store::try_session_lock(c, &g.session)?
    } else {
        store::lock(c, &g.session)?
    };
    // Recheck revocation after waiting on the lock.
    let revoked: i64 = fail(conn.query_row(
        "SELECT revoked FROM grants WHERE grant_id=?1",
        [&g.id],
        |r| r.get(0),
    ))?;
    if revoked != 0 {
        return Err("unauthorized");
    }
    let (op, state, wire_ref, baseline, ack): (String, String, String, String, Option<String>) =
        conn.query_row(
            "SELECT operation_id,state,wire,baseline,ack FROM operations WHERE request_id=?1",
            [&g.request],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
        )
        .optional()
        .map_err(|_| "bridge_unavailable")?
        .ok_or("unauthorized")?;
    if wire_ref != op {
        return Err("bridge_unavailable");
    }
    let wire = store::load_wire(c, &op, &g.wire_hash)?;
    if state == "admitted" {
        let tx = fail(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
        fail(tx.execute(
            "UPDATE operations SET state='admitted_no_attempt' WHERE request_id=?1",
            [&g.request],
        ))?;
        store::transition(&tx, &g.request, "admitted_no_attempt")?;
        fail(tx.commit())?;
    } else if state == "entry_uncertain" || state == "entered" {
        let mut temporary = None;
        if held.is_none() {
            temporary =
                Some(native::Native::connect(&c.native_socket).map_err(|_| "history_unavailable")?);
        }
        let n = held.or(temporary.as_mut()).ok_or("history_unavailable")?;
        n.verify_target(&g.session, &g.workspace)
            .map_err(|_| "history_unavailable")?;
        let history = n.history(&g.session).map_err(|_| "history_unavailable")?;
        if history != n.history(&g.session).map_err(|_| "history_unavailable")? {
            return Err("history_unavailable");
        }
        let previous: Vec<Value> = fail(serde_json::from_str(&baseline))?;
        let ids: std::collections::HashSet<&str> = previous
            .iter()
            .filter_map(|v| v.get("id").and_then(Value::as_str))
            .collect();
        if previous.iter().any(|old| {
            let id = old.get("id").and_then(Value::as_str);
            let hash = old.get("sha256").and_then(Value::as_str);
            !history.iter().any(|turn| {
                turn.get("id").and_then(Value::as_str) == id
                    && Some(
                        wire::digest(serde_json::to_string(turn).unwrap_or_default().as_bytes())
                            .as_str(),
                    ) == hash
            })
        }) {
            return Err("history_unavailable");
        }
        let mut matched = Vec::new();
        let mut conflict = ack.as_deref().is_some_and(|id| ids.contains(id));
        for t in &history {
            for item in t
                .get("items")
                .and_then(Value::as_array)
                .ok_or("history_unavailable")?
            {
                if item.get("type").and_then(Value::as_str) == Some("userMessage")
                    && item.get("clientId").and_then(Value::as_str) == Some(&op)
                {
                    let Some(contents) = item.get("content").and_then(Value::as_array) else {
                        conflict = true;
                        matched.push(t);
                        continue;
                    };
                    let users = t
                        .get("items")
                        .and_then(Value::as_array)
                        .ok_or("history_unavailable")?
                        .iter()
                        .filter(|item| {
                            item.get("type").and_then(Value::as_str) == Some("userMessage")
                        })
                        .count();
                    let exact = users == 1
                        && contents.len() == 1
                        && contents[0].get("type").and_then(Value::as_str) == Some("text")
                        && contents[0].get("text").and_then(Value::as_str) == Some(&wire);
                    if !exact || ids.contains(t.get("id").and_then(Value::as_str).unwrap_or("")) {
                        conflict = true;
                    }
                    matched.push(t);
                }
            }
        }
        if matched.len() > 1 {
            conflict = true;
        }
        if let Some(t) = matched.first() {
            let tid = t
                .get("id")
                .and_then(Value::as_str)
                .ok_or("identity_conflict")?;
            if ack.as_deref().is_some_and(|a| a != tid) {
                conflict = true;
            }
            if !conflict {
                let status = t
                    .get("status")
                    .and_then(Value::as_str)
                    .ok_or("history_unavailable")?;
                if !["inProgress", "completed", "failed", "interrupted"].contains(&status) {
                    return Err("history_unavailable");
                }
                let terminal = status != "inProgress";
                let tx = fail(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
                fail(tx.execute("UPDATE operations SET state=?1,turn_id=?2,native_status=?3,reconciled=?4 WHERE request_id=?5",params![if terminal {"terminal"} else {"entered"},tid,status,store::now(),g.request]))?;
                if terminal {
                    fail(tx.execute("DELETE FROM occupancy WHERE request_id=?1", [&g.request]))?;
                }
                store::transition(
                    &tx,
                    &g.request,
                    if terminal { "terminal" } else { "entered" },
                )?;
                fail(tx.commit())?;
            }
        }
        if matched.is_empty() && !conflict {
            fail(conn.execute(
                "UPDATE operations SET reconciled=?1 WHERE request_id=?2",
                params![store::now(), g.request],
            ))?;
        }
        if conflict {
            let tx = fail(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
            fail(tx.execute(
                "UPDATE operations SET state='identity_conflict',reconciled=?1 WHERE request_id=?2",
                params![store::now(), g.request],
            ))?;
            store::transition(&tx, &g.request, "identity_conflict")?;
            fail(tx.commit())?;
        }
    }
    checked_projection(conn, g)?.ok_or("bridge_unavailable")
}
pub fn run() {
    let result = cli(env::args().skip(1).collect());
    let value = match result {
        Ok(v) => {
            json!({"schema_version":1,"protocol":"session-bridge-rpc-v1","ok":true,"result":v})
        }
        Err(e) => {
            json!({"schema_version":1,"protocol":"session-bridge-rpc-v1","ok":false,"error":e})
        }
    };
    let _ = writeln!(io::stdout(), "{value}");
}
