//! Provisioning state and operator-issued, task-scoped authority.
use crate::{
    store::{self, Result},
    wire,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};
use uuid::Uuid;

fn fail<T, E>(v: std::result::Result<T, E>) -> Result<T> {
    v.map_err(|_| "bridge_unavailable")
}
fn hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn bounded(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 4096
        && !s.contains('\0')
        && !s.contains("weprovision_")
        && !s.contains("webridge_")
}

// This is operator input, never an MCP argument. It is stored verbatim with its
// digest so that neither an MCP caller nor later file edits can expand it.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema_version: u32,
    pub task_revision: String,
    pub profile_name: String,
    pub workspace: String,
    pub checkpoint: String,
    pub model: String,
    pub reasoning_effort: String,
    pub service_tier: String,
    pub sandbox: String,
    pub network_access: bool,
    pub approval_policy: String,
    pub global_config: String,
    pub global_config_sha256: String,
    pub workspace_config: String,
    pub workspace_config_sha256: String,
    pub request_id: String,
    pub payload_sha256: String,
    pub admit_until: i64,
    pub close_by: i64,
    pub budget_reference: String,
    pub budget_units: String,
    pub budget_amount: String,
    pub reserve_controller: String,
    pub reserve_amount: String,
    pub first_funded_clock: String,
}
impl Policy {
    fn validate(&self) -> Result<()> {
        if self.schema_version != 1
            || !wire::valid_uuid(&self.request_id)
            || !hash(&self.payload_sha256)
            || !hash(&self.global_config_sha256)
            || !hash(&self.workspace_config_sha256)
            || ![
                &self.task_revision,
                &self.profile_name,
                &self.checkpoint,
                &self.budget_reference,
                &self.budget_units,
                &self.budget_amount,
                &self.reserve_controller,
                &self.reserve_amount,
                &self.first_funded_clock,
                &self.workspace,
                &self.global_config,
                &self.workspace_config,
            ]
            .into_iter()
            .all(|s| bounded(s))
            || self.admit_until <= store::now()
            || self.close_by.saturating_sub(self.admit_until) < 15
            || self.model != "gpt-6.1-terra"
            || self.reasoning_effort != "medium"
            || self.service_tier != "default"
            || self.sandbox != "read-only"
            || self.network_access
            || self.approval_policy != "never"
            || !Path::new(&self.workspace).is_absolute()
            || !Path::new(&self.global_config).is_absolute()
            || !Path::new(&self.workspace_config).is_absolute()
        {
            return Err("bridge_unavailable");
        }
        Ok(())
    }
}

fn check_schema(conn: &Connection) -> Result<()> {
    let row: (String, i64) = fail(conn.query_row(
        "SELECT store_id,version FROM provision_meta",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    ))?;
    let count: i64 = fail(conn.query_row("SELECT count(*) FROM provision_meta", [], |r| r.get(0)))?;
    let store_id: String = fail(conn.query_row("SELECT id FROM meta", [], |r| r.get(0)))?;
    let objects:i64 = fail(conn.query_row("SELECT count(*) FROM sqlite_master WHERE (type='table' AND name IN ('provision_permissions','provision_transitions')) OR (type='trigger' AND name IN ('provision_insert_audit','provision_update_audit'))",[],|r|r.get(0)))?;
    if count != 1 || row.0 != store_id || row.1 != 1 || objects != 4 {
        return Err("bridge_unavailable");
    }
    Ok(())
}
pub fn initialize(conn: &mut Connection) -> Result<Value> {
    let tx = fail(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
    let present: i64 = fail(tx.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='provision_meta'",
        [],
        |r| r.get(0),
    ))?;
    if present == 0 {
        // Never adopt an unrelated or partially created table as our store.
        fail(tx.execute_batch("CREATE TABLE provision_meta (store_id TEXT NOT NULL, version INTEGER NOT NULL); CREATE TABLE provision_permissions (permission_id TEXT PRIMARY KEY, task_revision TEXT NOT NULL UNIQUE, token_hash TEXT NOT NULL UNIQUE, policy TEXT NOT NULL, policy_hash TEXT NOT NULL, revoked INTEGER NOT NULL DEFAULT 0, setup_key TEXT, setup_state TEXT NOT NULL DEFAULT 'unused', target_id TEXT, custody_generation TEXT, child_handle TEXT UNIQUE, child_grant_id TEXT, child_token TEXT, synthetic_state TEXT NOT NULL DEFAULT 'not_attempted', profile_verified_at INTEGER, closure_state TEXT NOT NULL DEFAULT 'not_requested', closure_observed_status TEXT, closure_observed_at INTEGER); CREATE TABLE provision_transitions (seq INTEGER PRIMARY KEY, permission_id TEXT NOT NULL, at INTEGER NOT NULL, setup_state TEXT NOT NULL, closure_state TEXT NOT NULL, synthetic_state TEXT NOT NULL, revoked INTEGER NOT NULL, target_id TEXT, custody_generation TEXT, child_handle TEXT, closure_observed_status TEXT, closure_observed_at INTEGER); CREATE TRIGGER provision_insert_audit AFTER INSERT ON provision_permissions BEGIN INSERT INTO provision_transitions(permission_id,at,setup_state,closure_state,synthetic_state,revoked,target_id,custody_generation,child_handle,closure_observed_status,closure_observed_at) VALUES (NEW.permission_id,unixepoch(),NEW.setup_state,NEW.closure_state,NEW.synthetic_state,NEW.revoked,NEW.target_id,NEW.custody_generation,NEW.child_handle,NEW.closure_observed_status,NEW.closure_observed_at); END; CREATE TRIGGER provision_update_audit AFTER UPDATE ON provision_permissions BEGIN INSERT INTO provision_transitions(permission_id,at,setup_state,closure_state,synthetic_state,revoked,target_id,custody_generation,child_handle,closure_observed_status,closure_observed_at) VALUES (NEW.permission_id,unixepoch(),NEW.setup_state,NEW.closure_state,NEW.synthetic_state,NEW.revoked,NEW.target_id,NEW.custody_generation,NEW.child_handle,NEW.closure_observed_status,NEW.closure_observed_at); END;"))?;
        fail(tx.execute("INSERT INTO provision_meta SELECT id,1 FROM meta", []))?;
    }
    check_schema(&tx)?;
    fail(tx.commit())?;
    Ok(json!({"schema_version":1,"provisioning_schema":1}))
}

pub fn issue(conn: &mut Connection, policy_file: &str, credential_out: &str) -> Result<Value> {
    check_schema(conn)?;
    let path = Path::new(policy_file);
    if !path.is_absolute() {
        return Err("bridge_unavailable");
    }
    store::secure(path, false)?;
    let mut data = Vec::new();
    fail(fs::File::open(path).and_then(|f| f.take(65537).read_to_end(&mut data)))?;
    if data.len() > 65536 {
        return Err("bridge_unavailable");
    }
    let policy: Policy = fail(serde_json::from_slice(&data))?;
    policy.validate()?;
    let canonical = fail(serde_json::to_string(&policy))?;
    let policy_hash = wire::digest(canonical.as_bytes());
    let id = Uuid::new_v4().to_string();
    let mut random = [0; 32];
    fail(getrandom::fill(&mut random))?;
    let credential = format!("weprovision_{}", URL_SAFE_NO_PAD.encode(random));
    let mut file = store::new_private(credential_out)?;
    let result = (|| {
        fail(writeln!(file, "{credential}"))?;
        fail(file.sync_all())?;
        fail(
            fs::File::open(
                Path::new(credential_out)
                    .parent()
                    .ok_or("bridge_unavailable")?,
            )
            .and_then(|f| f.sync_all()),
        )?;
        let tx = fail(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
        fail(tx.execute("INSERT INTO provision_permissions(permission_id,task_revision,token_hash,policy,policy_hash) VALUES (?1,?2,?3,?4,?5)",params![id,policy.task_revision,store::token_hash(&credential),canonical,policy_hash]))?;
        fail(tx.commit())?;
        Ok(
            json!({"schema_version":1,"permission_id":id,"policy_sha256":policy_hash,"admit_until":policy.admit_until,"close_by":policy.close_by,"scope":"one_target_one_initial_request","spend_settlement":"unavailable"}),
        )
    })();
    if result.is_err() {
        let _ = fs::remove_file(credential_out);
    }
    result
}

pub fn revoke(conn: &mut Connection, id: &str) -> Result<Value> {
    check_schema(conn)?;
    let tx = fail(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
    let grant: Option<Option<String>> = fail(
        tx.query_row(
            "SELECT child_grant_id FROM provision_permissions WHERE permission_id=?1",
            [id],
            |r| r.get(0),
        )
        .optional(),
    )?;
    let Some(grant) = grant else {
        return Err("unauthorized");
    };
    fail(tx.execute(
        "UPDATE provision_permissions SET revoked=1 WHERE permission_id=?1",
        [id],
    ))?;
    if let Some(id) = grant {
        fail(tx.execute("UPDATE grants SET revoked=1 WHERE grant_id=?1", [id]))?;
    }
    fail(tx.commit())?;
    Ok(json!({"schema_version":1,"permission_id":id,"revoked":true}))
}

pub struct Permission {
    pub id: String,
    pub policy: Policy,
    pub revoked: bool,
}
pub fn authenticate(conn: &Connection, token: &str, allow_revoked: bool) -> Result<Permission> {
    check_schema(conn)?;
    let row: Option<(String,String,String,i64)> = fail(conn.query_row("SELECT permission_id,policy,policy_hash,revoked FROM provision_permissions WHERE token_hash=?1",[store::token_hash(token)],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional())?;
    let Some((id, raw, expected, revoked)) = row else {
        return Err("unauthorized");
    };
    if wire::digest(raw.as_bytes()) != expected || (revoked != 0 && !allow_revoked) {
        return Err("unauthorized");
    }
    let policy: Policy = fail(serde_json::from_str(&raw))?;
    Ok(Permission {
        id,
        policy,
        revoked: revoked != 0,
    })
}

fn file_hash(path: &str) -> Result<String> {
    let m = fail(fs::metadata(path))?;
    if !m.is_file() || m.len() > 1024 * 1024 {
        return Err("target_mismatch");
    }
    let mut bytes = Vec::new();
    fail(fs::File::open(path).and_then(|f| f.take(1024 * 1024 + 1).read_to_end(&mut bytes)))?;
    if bytes.len() > 1024 * 1024 {
        return Err("target_mismatch");
    }
    Ok(wire::digest(&bytes))
}
fn check_files(policy: &Policy) -> Result<()> {
    let workspace = fail(fs::canonicalize(&policy.workspace))?;
    if workspace.to_str() != Some(&policy.workspace)
        || !workspace.is_dir()
        || Path::new(&policy.workspace_config) != workspace.join(".codex/config.toml")
        || fail(fs::canonicalize(&policy.workspace_config))? != Path::new(&policy.workspace_config)
        || fail(fs::canonicalize(&policy.global_config))? != Path::new(&policy.global_config)
        || file_hash(&policy.workspace_config)? != policy.workspace_config_sha256
        || file_hash(&policy.global_config)? != policy.global_config_sha256
    {
        return Err("target_mismatch");
    }
    Ok(())
}
fn flags_off(value: &Value, required: &[&str]) -> bool {
    let Some(map) = value.as_object() else {
        return false;
    };
    required
        .iter()
        .all(|k| map.get(*k).and_then(Value::as_bool) == Some(false))
}
fn maps_disabled(value: &Value, required: &[&str]) -> bool {
    let Some(map) = value.as_object() else {
        return false;
    };
    required.iter().all(|k| {
        map.get(*k)
            .and_then(|v| v.get("enabled"))
            .and_then(Value::as_bool)
            == Some(false)
    }) && map
        .values()
        .all(|v| v.get("enabled").and_then(Value::as_bool) == Some(false))
}
const FLAGS: &[&str] = &[
    "apps",
    "plugins",
    "remote_plugin",
    "multi_agent",
    "multi_agent_v2",
];
const SERVERS: &[&str] = &["codebase-memory-mcp", "cua_repl", "node_repl"];
const PLUGINS: &[&str] = &[
    "browser@openai-bundled",
    "chrome@openai-bundled",
    "code-review@openai-bundled",
    "codex-app-tools@openai-bundled",
    "documents@openai-primary-runtime",
    "github@openai-curated",
    "pdf@openai-primary-runtime",
    "presentations@openai-primary-runtime",
    "spreadsheets@openai-primary-runtime",
    "template-creator@openai-primary-runtime",
    "unified-computer-use@openai-bundled",
    "visualize@openai-bundled",
];
fn config_check(native: &mut crate::native::Native, policy: &Policy) -> Result<()> {
    check_files(policy)?;
    let r = native
        .call(
            "config/read",
            json!({"cwd":policy.workspace,"includeLayers":true}),
        )
        .map_err(|_| "history_unavailable")?;
    let config = r.get("config").ok_or("target_mismatch")?;
    let layers = r
        .get("layers")
        .and_then(Value::as_array)
        .ok_or("target_mismatch")?;
    let expected_folder = format!("{}/.codex", policy.workspace);
    if !r.get("origins").is_some_and(Value::is_object)
        || layers
            .iter()
            .filter(|l| {
                l.pointer("/name/type").and_then(Value::as_str) == Some("project")
                    && l.pointer("/name/dotCodexFolder").and_then(Value::as_str)
                        == Some(expected_folder.as_str())
                    && l.get("disabledReason").is_none_or(Value::is_null)
            })
            .count()
            != 1
        || layers
            .iter()
            .filter(|l| {
                l.pointer("/name/type").and_then(Value::as_str) == Some("user")
                    && l.pointer("/name/file").and_then(Value::as_str)
                        == Some(policy.global_config.as_str())
                    && l.get("disabledReason").is_none_or(Value::is_null)
            })
            .count()
            != 1
        || config.get("model").and_then(Value::as_str) != Some(policy.model.as_str())
        || config.get("model_reasoning_effort").and_then(Value::as_str)
            != Some(policy.reasoning_effort.as_str())
        || config.get("service_tier").and_then(Value::as_str) != Some(policy.service_tier.as_str())
        || config.get("approval_policy").and_then(Value::as_str)
            != Some(policy.approval_policy.as_str())
        || config.get("sandbox_mode").and_then(Value::as_str) != Some(policy.sandbox.as_str())
        || config.get("web_search").and_then(Value::as_str) != Some("disabled")
        || !flags_off(&config["features"], FLAGS)
        || !maps_disabled(&config["mcp_servers"], SERVERS)
        || !maps_disabled(&config["plugins"], PLUGINS)
        || !maps_disabled(&config["apps"], &["_default"])
    {
        return Err("target_mismatch");
    }
    check_files(policy)
}
pub(crate) fn target_ack(r: &Value, p: &Policy, expected: Option<&str>) -> Result<String> {
    let id = r
        .pointer("/thread/id")
        .and_then(Value::as_str)
        .filter(|s| wire::valid_uuid(s))
        .ok_or("target_mismatch")?;
    if expected.is_some_and(|v| v != id)
        || r.pointer("/thread/cwd").and_then(Value::as_str) != Some(p.workspace.as_str())
        || r.pointer("/thread/status/type").and_then(Value::as_str) != Some("idle")
        || r.get("model").and_then(Value::as_str) != Some(p.model.as_str())
        || r.get("reasoningEffort").and_then(Value::as_str) != Some(p.reasoning_effort.as_str())
        || r.get("serviceTier").and_then(Value::as_str) != Some(p.service_tier.as_str())
        || r.get("approvalPolicy").and_then(Value::as_str) != Some(p.approval_policy.as_str())
        || r.pointer("/sandbox/type").and_then(Value::as_str) != Some("readOnly")
        || r.pointer("/sandbox/networkAccess").and_then(Value::as_bool) != Some(false)
    {
        return Err("target_mismatch");
    }
    Ok(id.to_owned())
}
fn features(native: &mut crate::native::Native, sid: &str) -> Result<()> {
    use std::collections::{HashMap, HashSet};
    let mut values = HashMap::new();
    let mut seen = HashSet::new();
    let mut cursor = None;
    for _ in 0..100 {
        let mut p = json!({"threadId":sid,"limit":100});
        if let Some(c) = &cursor {
            p["cursor"] = json!(c);
        }
        let r = native
            .call("experimentalFeature/list", p)
            .map_err(|_| "history_unavailable")?;
        for f in r
            .get("data")
            .and_then(Value::as_array)
            .ok_or("target_mismatch")?
        {
            let name = f
                .get("name")
                .and_then(Value::as_str)
                .ok_or("target_mismatch")?;
            let enabled = f
                .get("enabled")
                .and_then(Value::as_bool)
                .ok_or("target_mismatch")?;
            if values.insert(name.to_owned(), enabled).is_some() {
                return Err("target_mismatch");
            }
        }
        match r.get("nextCursor") {
            Some(Value::Null) => {
                return if FLAGS.iter().all(|f| values.get(*f) == Some(&false)) {
                    Ok(())
                } else {
                    Err("target_mismatch")
                };
            }
            Some(Value::String(c)) if !c.is_empty() && seen.insert(c.clone()) => {
                cursor = Some(c.clone())
            }
            _ => return Err("history_unavailable"),
        }
    }
    Err("history_unavailable")
}

// Git is invoked only for the operator-pinned workspace and fixed arguments.
// No shell or MCP-supplied program/arguments are accepted.
fn checkpoint(policy: &Policy, deadline: std::time::Instant) -> Result<()> {
    use std::{
        os::unix::process::CommandExt,
        process::{Command, Stdio},
        time::Duration,
    };
    let deadline = deadline.min(std::time::Instant::now() + Duration::from_secs(10));
    if ![40, 64].contains(&policy.checkpoint.len())
        || !policy
            .checkpoint
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err("target_mismatch");
    }
    for (args, expected) in [
        (
            &["config", "--get-regexp", "^filter\\."][..],
            Some("__NO_FILTER_CONFIG__"),
        ),
        (
            &["rev-parse", "--verify", "HEAD"][..],
            Some(policy.checkpoint.as_str()),
        ),
        (
            &[
                "status",
                "--porcelain",
                "-z",
                "--ignored",
                "--untracked-files=all",
            ][..],
            None,
        ),
    ] {
        let mut child = fail(
            Command::new("/usr/bin/git")
                .arg("-C")
                .arg(&policy.workspace)
                .args([
                    "--no-optional-locks",
                    "-c",
                    "core.fsmonitor=false",
                    "-c",
                    "core.hooksPath=/dev/null",
                ])
                .args(args)
                .env_clear()
                .process_group(0)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .env("GIT_CONFIG_GLOBAL", "/dev/null")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn(),
        )?;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let mut output = Vec::new();
                    fail(
                        child
                            .stdout
                            .take()
                            .ok_or("bridge_unavailable")?
                            .take(1025)
                            .read_to_end(&mut output),
                    )?;
                    if output.len() > 1024 {
                        return Err("target_mismatch");
                    }
                    if expected == Some("__NO_FILTER_CONFIG__") {
                        if status.code() != Some(1) || !output.is_empty() {
                            return Err("target_mismatch");
                        }
                        break;
                    }
                    if !status.success() {
                        return Err("target_mismatch");
                    }
                    match expected {
                        Some(expected)
                            if std::str::from_utf8(&output).ok().map(str::trim)
                                == Some(expected) => {}
                        None if output.is_empty() || output == b"!! .codex/config.toml\0" => {}
                        _ => return Err("target_mismatch"),
                    }
                    break;
                }
                Ok(None) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                _ => {
                    // The fixed Git command may launch helpers (for example,
                    // submodule inspection). Own its process group on timeout.
                    unsafe {
                        libc::kill(-(child.id() as i32), libc::SIGKILL);
                    }
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("target_mismatch");
                }
            }
        }
    }
    Ok(())
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerRpc {
    pub method: String,
    pub provisioning_credential: String,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub grant_handle: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub workspace: Option<String>,
    #[serde(default)]
    pub input_text: Option<String>,
}
struct Owned {
    permission_id: String,
    target: String,
    generation: String,
    native: crate::native::Native,
}
fn deadline(unix: i64) -> Result<std::time::Instant> {
    let nanos =
        i128::from(unix) * 1_000_000_000 - time::OffsetDateTime::now_utc().unix_timestamp_nanos();
    let nanos = u64::try_from(nanos)
        .ok()
        .filter(|v| *v > 0)
        .ok_or("expired")?;
    std::time::Instant::now()
        .checked_add(std::time::Duration::from_nanos(nanos))
        .ok_or("expired")
}
type ProjectionRow = (
    Option<String>,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
    Option<i64>,
    String,
    String,
);
pub(crate) fn projection(conn: &Connection, id: &str) -> Result<Value> {
    let (key,state,target,generation,handle,closure,synthetic,verified,raw,hash):ProjectionRow=fail(conn.query_row("SELECT setup_key,setup_state,target_id,custody_generation,child_handle,closure_state,synthetic_state,profile_verified_at,policy,policy_hash FROM provision_permissions WHERE permission_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?))))?;
    if wire::digest(raw.as_bytes()) != hash {
        return Err("bridge_unavailable");
    }
    let p: Policy = fail(serde_json::from_str(&raw))?;
    let audit_count: i64 = fail(conn.query_row(
        "SELECT count(*) FROM provision_transitions WHERE permission_id=?1",
        [id],
        |r| r.get(0),
    ))?;
    let (observed_status,observed_at):(Option<String>,Option<i64>) = fail(conn.query_row("SELECT closure_observed_status,closure_observed_at FROM provision_permissions WHERE permission_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?))))?;
    let effective = if verified.is_some() {
        json!({"model":p.model,"reasoning_effort":p.reasoning_effort,"service_tier":p.service_tier,"sandbox":p.sandbox,"network_access":p.network_access,"approval_policy":p.approval_policy,"workspace_config_sha256":p.workspace_config_sha256,"global_config_sha256":p.global_config_sha256,"disabled_features":FLAGS,"disabled_mcp_servers":SERVERS,"disabled_plugins":PLUGINS,"apps_default_disabled":true,"verified_at":verified})
    } else {
        Value::Null
    };
    Ok(
        json!({"schema_version":1,"effective_profile":effective,"closure_observation":{"status":observed_status,"observed_at":observed_at},"audit_transition_count":audit_count,"permission_id":id,"idempotency_key":key,"setup_state":state,"target_id":target,"custody_generation":generation,"grant_handle":handle,"closure_state":closure,"synthetic_initialization":synthetic,"profile_verified_at":verified,"policy_sha256":hash,"task_revision":p.task_revision,"profile":p.profile_name,"workspace":p.workspace,"checkpoint":p.checkpoint,"admit_until":p.admit_until,"close_by":p.close_by,"budget_reference":p.budget_reference,"budget_units":p.budget_units,"budget_amount":p.budget_amount,"reserve_controller":p.reserve_controller,"reserve_amount":p.reserve_amount,"first_funded_clock":p.first_funded_clock,"spend_settlement":"unavailable","limitations":["held owner does not fence other native clients","provider spend and settlement unavailable","first funded clock is supervisor observed"]}),
    )
}
fn reserve(conn: &mut Connection, p: &Permission, key: &str) -> Result<Value> {
    if !wire::valid_uuid(key) || p.revoked {
        return Err("unauthorized");
    }
    let tx = fail(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
    let (old, state, revoked): (Option<String>, String, i64) = fail(tx.query_row(
        "SELECT setup_key,setup_state,revoked FROM provision_permissions WHERE permission_id=?1",
        [&p.id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ))?;
    if revoked != 0 {
        return Err("unauthorized");
    }
    if let Some(old) = old {
        if old != key {
            return Err("payload_conflict");
        }
    } else {
        if store::now() >= p.policy.admit_until {
            return Err("expired");
        }
        if state != "unused" {
            return Err("bridge_unavailable");
        }
        fail(tx.execute("UPDATE provision_permissions SET setup_key=?1,setup_state='allocation_uncertain' WHERE permission_id=?2",params![key,p.id]))?;
    }
    fail(tx.commit())?;
    projection(conn, &p.id)
}
fn set_state(
    conn: &Connection,
    id: &str,
    state: &str,
    target: Option<&str>,
    generation: Option<&str>,
) -> Result<()> {
    fail(conn.execute("UPDATE provision_permissions SET setup_state=?1,target_id=COALESCE(?2,target_id),custody_generation=?3 WHERE permission_id=?4",params![state,target,generation,id]))?;
    Ok(())
}
fn create(
    c: &crate::wire::Config,
    conn: &mut Connection,
    p: &Permission,
    key: &str,
    owned: &mut Option<Owned>,
) -> Result<Value> {
    // Repeated keys never repeat allocation, including if the first ACK was lost.
    let existing = projection(conn, &p.id)?;
    if existing["idempotency_key"].as_str().is_some() {
        return reserve(conn, p, key);
    }
    if owned.is_some() {
        return Err("session_busy");
    }
    let unresolved:i64=fail(conn.query_row("SELECT count(*) FROM provision_permissions WHERE permission_id!=?1 AND setup_state NOT IN ('unused','closed','refused_before_allocation')",[&p.id],|r|r.get(0)))?;
    if unresolved != 0 {
        return Err("session_busy");
    }
    if p.revoked || store::now() >= p.policy.admit_until {
        return Err("expired");
    }
    check_files(&p.policy)?;
    let until = deadline(p.policy.close_by)?;
    checkpoint(&p.policy, until)?;
    let mut native = crate::native::Native::connect_with_deadline(&c.native_socket, until, true)
        .map_err(|_| "history_unavailable")?;
    config_check(&mut native, &p.policy)?;
    reserve(conn, p, key)?; // durable uncertain intent precedes the one external allocation
    if store::now() >= p.policy.admit_until {
        set_state(conn, &p.id, "refused_before_allocation", None, None)?;
        return projection(conn, &p.id);
    }
    check_files(&p.policy)?;
    checkpoint(&p.policy, deadline(p.policy.admit_until)?)?;
    let result=native.call_until("thread/start",json!({"cwd":p.policy.workspace,"model":p.policy.model,"sandbox":"read-only","approvalPolicy":"never","historyMode":"legacy"}),deadline(p.policy.admit_until)?);
    let Ok(response) = result else {
        return projection(conn, &p.id);
    };
    let Some(id) = response
        .pointer("/thread/id")
        .and_then(Value::as_str)
        .filter(|s| wire::valid_uuid(s))
        .map(str::to_owned)
    else {
        return projection(conn, &p.id);
    };
    set_state(conn, &p.id, "identified", Some(&id), None)?;
    let generation = Uuid::new_v4().to_string();
    *owned = Some(Owned {
        permission_id: p.id.clone(),
        target: id.clone(),
        generation: generation.clone(),
        native,
    });
    let holder = owned.as_mut().ok_or("bridge_unavailable")?;
    if target_ack(&response, &p.policy, Some(&id)).is_err() {
        return projection(conn, &p.id);
    }
    if features(&mut holder.native, &id).is_err()
        || config_check(&mut holder.native, &p.policy).is_err()
    {
        return projection(conn, &p.id);
    }
    let history = match holder.native.preflight(&id, &p.policy.workspace) {
        Ok(h) if h.is_empty() => {
            fail(conn.execute("UPDATE provision_permissions SET synthetic_state='not_needed' WHERE permission_id=?1",[&p.id]))?;
            h
        }
        Err("history_unavailable") => {
            if holder
                .native
                .verify_target(&id, &p.policy.workspace)
                .is_err()
            {
                return projection(conn, &p.id);
            }
            let thread = holder.native.call("thread/read", json!({"threadId":id}));
            if !thread.is_ok_and(|v| {
                v.pointer("/thread/status/type").and_then(Value::as_str) == Some("idle")
            }) {
                return projection(conn, &p.id);
            }
            fail(conn.execute("UPDATE provision_permissions SET synthetic_state='uncertain' WHERE permission_id=?1",[&p.id]))?;
            set_state(
                conn,
                &p.id,
                "initialization_uncertain",
                Some(&id),
                Some(&generation),
            )?;
            let Some(until) = deadline(p.policy.admit_until).ok() else {
                return projection(conn, &p.id);
            };
            if holder.native.call_until("thread/inject_items",json!({"threadId":id,"items":[{"type":"message","role":"assistant","content":[{"type":"output_text","text":""}]}]}),until).is_err() { return projection(conn,&p.id); }
            fail(conn.execute("UPDATE provision_permissions SET synthetic_state='one_empty_assistant_output_text' WHERE permission_id=?1",[&p.id]))?;
            match holder.native.preflight(&id, &p.policy.workspace) {
                Ok(h) if h.is_empty() => h,
                _ => return projection(conn, &p.id),
            }
        }
        _ => return projection(conn, &p.id),
    };
    let Some(until) = deadline(p.policy.admit_until).ok() else {
        return projection(conn, &p.id);
    };
    let resume = holder
        .native
        .call_until("thread/resume", json!({"threadId":id}), until);
    if !resume.is_ok_and(|v| target_ack(&v, &p.policy, Some(&id)).is_ok())
        || holder
            .native
            .preflight(&id, &p.policy.workspace)
            .ok()
            .as_ref()
            != Some(&history)
        || features(&mut holder.native, &id).is_err()
        || config_check(&mut holder.native, &p.policy).is_err()
        || store::now() >= p.policy.admit_until
    {
        return projection(conn, &p.id);
    }
    set_state(conn, &p.id, "ready", Some(&id), Some(&generation))?;
    fail(conn.execute(
        "UPDATE provision_permissions SET profile_verified_at=?1 WHERE permission_id=?2",
        params![store::now(), p.id],
    ))?;
    projection(conn, &p.id)
}

fn healthy(
    conn: &Connection,
    p: &Permission,
    owned: &mut Option<Owned>,
    admission: bool,
) -> Result<()> {
    if p.revoked {
        return Err("unauthorized");
    }
    if admission && store::now() >= p.policy.admit_until {
        return Err("expired");
    }
    if store::now() >= p.policy.close_by {
        return Err("expired");
    }
    let o = owned
        .as_mut()
        .filter(|o| o.permission_id == p.id)
        .ok_or("custody_lost")?;
    let row:(String,Option<String>,Option<String>,i64)=fail(conn.query_row("SELECT setup_state,target_id,custody_generation,revoked FROM provision_permissions WHERE permission_id=?1",[&p.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))))?;
    if row.0 != "ready"
        || row.1.as_deref() != Some(&o.target)
        || row.2.as_deref() != Some(&o.generation)
        || row.3 != 0
    {
        return Err("custody_lost");
    }
    checkpoint(&p.policy, deadline(p.policy.close_by)?)?;
    config_check(&mut o.native, &p.policy)?;
    features(&mut o.native, &o.target)?;
    o.native.preflight(&o.target, &p.policy.workspace)?;
    if admission && store::now() >= p.policy.admit_until {
        return Err("expired");
    }
    Ok(())
}
pub(crate) fn recheck_at_entry(
    native: &mut crate::native::Native,
    p: &Policy,
    target: &str,
) -> Result<()> {
    checkpoint(p, deadline(p.admit_until)?)?;
    config_check(native, p)?;
    features(native, target)?;
    if store::now() >= p.admit_until {
        return Err("expired");
    }
    Ok(())
}
fn issue_child(
    conn: &mut Connection,
    p: &Permission,
    text: &str,
    owned: &mut Option<Owned>,
) -> Result<Value> {
    if text.contains("weprovision_")
        || text.contains("webridge_")
        || wire::digest(text.as_bytes()) != p.policy.payload_sha256
    {
        return Err("payload_conflict");
    }
    if p.revoked || store::now() >= p.policy.admit_until {
        return Err("expired");
    }
    let existing: Option<(String,String)> = fail(conn.query_row("SELECT child_handle,target_id FROM provision_permissions WHERE permission_id=?1 AND revoked=0 AND setup_state='ready' AND child_handle IS NOT NULL",[&p.id],|r|Ok((r.get(0)?,r.get(1)?))).optional())?;
    if let Some((handle, target)) = existing {
        let grant = child_grant(conn, p, &handle, &p.policy.request_id)?;
        let (_, _, expected) = wire::wire(&p.policy.request_id, &target, &p.policy.workspace, text)
            .map_err(|_| "payload_conflict")?;
        if grant.wire_hash != expected {
            return Err("payload_conflict");
        }
        return Ok(
            json!({"schema_version":1,"grant_handle":handle,"request_id":p.policy.request_id,"target_id":target,"wire_sha256":grant.wire_hash,"capability_scope":"message_admission_only"}),
        );
    }
    healthy(conn, p, owned, true)?;
    let o = owned.as_ref().ok_or("custody_lost")?;
    let (_, ph, wh) = wire::wire(&p.policy.request_id, &o.target, &p.policy.workspace, text)
        .map_err(|_| "payload_conflict")?;
    let tx = fail(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
    let row:(Option<String>,Option<String>,i64,String,Option<String>)=fail(tx.query_row("SELECT child_handle,child_grant_id,revoked,setup_state,custody_generation FROM provision_permissions WHERE permission_id=?1",[&p.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))))?;
    if row.2 != 0 || row.3 != "ready" || row.4.as_deref() != Some(&o.generation) {
        return Err("unauthorized");
    }
    if store::now() >= p.policy.admit_until {
        return Err("expired");
    }
    if row.0.is_some() || row.1.is_some() {
        return Err("bridge_unavailable");
    }
    let mut random = [0; 32];
    fail(getrandom::fill(&mut random))?;
    let token = format!("webridge_{}", URL_SAFE_NO_PAD.encode(random));
    let gid = Uuid::new_v4().to_string();
    let handle = Uuid::new_v4().to_string();
    let op = wire::operation_digest(
        &p.policy.request_id,
        &o.target,
        &p.policy.workspace,
        &ph,
        &wh,
    );
    tx.execute("INSERT INTO grants (grant_id,token_hash,request_id,session_id,workspace,payload_hash,wire_hash,operation_hash,expires,issuer_uid,observed_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![gid,store::token_hash(&token),p.policy.request_id,o.target,p.policy.workspace,ph,wh,op,p.policy.admit_until,unsafe{libc::geteuid()},store::now()]).map_err(|_|"payload_conflict")?;
    fail(tx.execute("UPDATE provision_permissions SET child_handle=?1,child_grant_id=?2,child_token=?3 WHERE permission_id=?4",params![handle,gid,token,p.id]))?;
    fail(tx.commit())?;
    Ok(
        json!({"schema_version":1,"grant_handle":handle,"request_id":p.policy.request_id,"target_id":o.target,"wire_sha256":wh,"capability_scope":"message_admission_only"}),
    )
}
fn child_grant(
    conn: &Connection,
    p: &Permission,
    handle: &str,
    request: &str,
) -> Result<store::Grant> {
    if p.revoked || request != p.policy.request_id {
        return Err("unauthorized");
    }
    let row:Option<(String,String,String)>=fail(conn.query_row("SELECT child_token,target_id,child_grant_id FROM provision_permissions WHERE permission_id=?1 AND child_handle=?2 AND revoked=0",params![p.id,handle],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional())?;
    let (token, target, id) = row.ok_or("unauthorized")?;
    let grant = store::authorize(conn, request, &token)?;
    if grant.id != id
        || grant.session != target
        || grant.workspace != p.policy.workspace
        || grant.payload_hash != p.policy.payload_sha256
    {
        return Err("unauthorized");
    }
    Ok(grant)
}

fn observe_closure(
    conn: &Connection,
    permission: &str,
    expected_id: &str,
    workspace: &str,
    response: &Value,
) -> Result<Option<String>> {
    let Some(thread) = response.get("thread") else {
        return Ok(None);
    };
    if thread.get("id").and_then(Value::as_str) != Some(expected_id)
        || thread.get("cwd").and_then(Value::as_str) != Some(workspace)
    {
        return Ok(None);
    }
    let Some(status) = thread
        .pointer("/status/type")
        .and_then(Value::as_str)
        .filter(|s| ["idle", "active", "notLoaded"].contains(s))
    else {
        return Ok(None);
    };
    fail(conn.execute("UPDATE provision_permissions SET closure_observed_status=?1,closure_observed_at=?2 WHERE permission_id=?3",params![status,store::now(),permission]))?;
    Ok(Some(status.to_owned()))
}
fn close(
    c: &crate::wire::Config,
    conn: &mut Connection,
    p: &Permission,
    owned: &mut Option<Owned>,
) -> Result<Value> {
    let current = projection(conn, &p.id)?;
    let Some(id) = current["target_id"].as_str().map(str::to_owned) else {
        // A never-allocated slot can be closed. An unknown allocation stays
        // uncertain and is never retried or relabelled as closed.
        let unused = matches!(
            current["setup_state"].as_str(),
            Some("unused" | "refused_before_allocation")
        );
        let tx = fail(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
        fail(tx.execute("UPDATE provision_permissions SET setup_state=CASE WHEN ?1 THEN 'closed' ELSE setup_state END,closure_state=CASE WHEN ?1 THEN 'closed' ELSE 'uncertain' END WHERE permission_id=?2",params![unused,p.id]))?;
        fail(tx.commit())?;
        return projection(conn, &p.id);
    };
    let previous = current["closure_state"]
        .as_str()
        .ok_or("bridge_unavailable")?
        .to_owned();
    let tx = fail(conn.transaction_with_behavior(TransactionBehavior::Immediate))?;
    let grant: Option<String> = fail(tx.query_row(
        "SELECT child_grant_id FROM provision_permissions WHERE permission_id=?1",
        [&p.id],
        |r| r.get(0),
    ))?;
    if let Some(g) = grant {
        fail(tx.execute("UPDATE grants SET revoked=1 WHERE grant_id=?1", [g]))?;
    }
    if previous == "not_requested" {
        fail(tx.execute("UPDATE provision_permissions SET closure_state='pending',setup_state='closing' WHERE permission_id=?1",[&p.id]))?;
    }
    fail(tx.commit())?;
    if previous == "closed" {
        return projection(conn, &p.id);
    }
    let _lock = match store::try_session_lock(c, &id) {
        Ok(lock) => lock,
        Err(_) => return projection(conn, &p.id),
    };
    // Cleanup gets a separate short allowance. A lost archive ACK is only read
    // back; it is never automatically replayed.
    let until = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut temporary = None;
    let use_held = store::now() < p.policy.close_by
        && owned
            .as_ref()
            .is_some_and(|o| o.permission_id == p.id && o.target == id);
    if !use_held {
        temporary =
            crate::native::Native::connect_with_deadline(&c.native_socket, until, true).ok();
    }
    let native = if let Some(o) = owned
        .as_mut()
        .filter(|o| use_held && o.permission_id == p.id && o.target == id)
    {
        o.native.tighten_deadline(until);
        Some(&mut o.native)
    } else {
        temporary.as_mut()
    };
    let mut connection_failed = false;
    if let Some(n) = native {
        let before = n.call("thread/read", json!({"threadId":id}));
        if before.is_err() {
            connection_failed = true;
        }
        let checked = before.as_ref().ok().and_then(|v| v.get("thread"));
        let identity = checked.is_some_and(|t| {
            t.get("id").and_then(Value::as_str) == Some(id.as_str())
                && t.get("cwd").and_then(Value::as_str) == Some(p.policy.workspace.as_str())
        });
        let before_status = match before.as_ref() {
            Ok(value) => observe_closure(conn, &p.id, &id, &p.policy.workspace, value)?,
            Err(_) => None,
        };
        let mut closed =
            previous == "archive_acknowledged" && before_status.as_deref() == Some("notLoaded");
        if !closed
            && identity
            && (previous == "not_requested" || previous == "pending")
            && n.verify_target(&id, &p.policy.workspace).is_ok()
        {
            fail(conn.execute(
                "UPDATE provision_permissions SET closure_state='uncertain' WHERE permission_id=?1",
                [&p.id],
            ))?;
            let archive = n.call("thread/archive", json!({"threadId":id}));
            if archive.is_ok() {
                fail(conn.execute("UPDATE provision_permissions SET closure_state='archive_acknowledged' WHERE permission_id=?1",[&p.id]))?;
            }
            let observation = n.call("thread/read", json!({"threadId":id}));
            if archive.is_err() || observation.is_err() {
                connection_failed = true;
            }
            let after_status = match observation.as_ref() {
                Ok(value) => observe_closure(conn, &p.id, &id, &p.policy.workspace, value)?,
                Err(_) => None,
            };
            closed = archive.is_ok() && after_status.as_deref() == Some("notLoaded");
        }
        if closed {
            fail(conn.execute("UPDATE provision_permissions SET closure_state='closed',setup_state='closed',custody_generation=NULL WHERE permission_id=?1",[&p.id]))?;
            if owned.as_ref().is_some_and(|o| o.permission_id == p.id) {
                *owned = None;
            }
        }
    }
    if connection_failed && owned.as_ref().is_some_and(|o| o.permission_id == p.id) {
        *owned = None;
    }
    projection(conn, &p.id)
}
fn owner_call(
    c: &crate::wire::Config,
    conn: &mut Connection,
    request: OwnerRpc,
    owned: &mut Option<Owned>,
) -> Result<Value> {
    let closing = request.method == "close_session_target";
    let p = authenticate(conn, &request.provisioning_credential, closing)?;
    let _permission_lock = store::permission_lock(c, &p.id)?;
    let p = authenticate(conn, &request.provisioning_credential, closing)?;
    let no_extra = |r: &OwnerRpc| {
        r.idempotency_key.is_none()
            && r.grant_handle.is_none()
            && r.request_id.is_none()
            && r.session_id.is_none()
            && r.workspace.is_none()
            && r.input_text.is_none()
    };
    match request.method.as_str() {
        "create_session_target"
            if request.grant_handle.is_none()
                && request.request_id.is_none()
                && request.session_id.is_none()
                && request.workspace.is_none()
                && request.input_text.is_none() =>
        {
            create(
                c,
                conn,
                &p,
                request.idempotency_key.as_deref().ok_or("unauthorized")?,
                owned,
            )
        }
        "read_session_target" if no_extra(&request) => {
            let present = owned.as_ref().is_some_and(|o| o.permission_id == p.id);
            let alive = owned
                .as_mut()
                .filter(|o| o.permission_id == p.id)
                .is_some_and(|o| {
                    o.native
                        .verify_target(&o.target, &p.policy.workspace)
                        .is_ok()
                });
            let health = if alive {
                "held"
            } else if present {
                fail(conn.execute("UPDATE provision_permissions SET setup_state=CASE WHEN setup_state='ready' THEN 'custody_lost' ELSE setup_state END,custody_generation=NULL WHERE permission_id=?1",[&p.id]))?;
                *owned = None;
                "custody_unverified"
            } else {
                "no_held_target"
            };
            let mut v = projection(conn, &p.id)?;
            v["owner_health"] = json!(health);
            Ok(v)
        }
        "close_session_target" if no_extra(&request) => close(c, conn, &p, owned),
        "issue_session_request_grant"
            if request.idempotency_key.is_none()
                && request.grant_handle.is_none()
                && request.request_id.is_none()
                && request.session_id.is_none()
                && request.workspace.is_none() =>
        {
            issue_child(
                conn,
                &p,
                request.input_text.as_deref().ok_or("unauthorized")?,
                owned,
            )
        }
        "submit_session_request" => {
            if request.idempotency_key.is_some() {
                return Err("unauthorized");
            }
            let g = child_grant(
                conn,
                &p,
                request.grant_handle.as_deref().ok_or("unauthorized")?,
                request.request_id.as_deref().ok_or("unauthorized")?,
            )?;
            if request.session_id.as_deref() != Some(&g.session)
                || request.workspace.as_deref() != Some(&g.workspace)
            {
                return Err("target_mismatch");
            }
            let text = request.input_text.as_deref().ok_or("unauthorized")?;
            let (wire, ph, wh) = wire::wire(&g.request, &g.session, &g.workspace, text)
                .map_err(|_| "payload_conflict")?;
            if ph != g.payload_hash
                || wh != g.wire_hash
                || wire::operation_digest(&g.request, &g.session, &g.workspace, &ph, &wh)
                    != g.operation_hash
            {
                return Err("payload_conflict");
            }
            if store::now() >= p.policy.admit_until {
                return Err("expired");
            }
            if let Some(existing) = crate::checked_projection(conn, &g)? {
                return Ok(existing);
            }
            healthy(conn, &p, owned, true)?;
            let o = owned.as_mut().ok_or("custody_lost")?;
            crate::submit_with_native(
                c,
                conn,
                &g,
                &wire,
                Some(&mut o.native),
                Some(deadline(p.policy.admit_until)?),
                Some(&p.policy),
            )
        }
        "read_session_operation" | "read_session_result" => {
            if request.idempotency_key.is_some()
                || request.session_id.is_some()
                || request.workspace.is_some()
                || request.input_text.is_some()
            {
                return Err("unauthorized");
            }
            let g = child_grant(
                conn,
                &p,
                request.grant_handle.as_deref().ok_or("unauthorized")?,
                request.request_id.as_deref().ok_or("unauthorized")?,
            )?;
            let result = (|| -> Result<Value> {
                let mut temporary = None;
                if owned
                    .as_ref()
                    .is_none_or(|o| o.permission_id != p.id || o.target != g.session)
                {
                    temporary = Some(
                        crate::native::Native::connect_with_deadline(
                            &c.native_socket,
                            deadline(p.policy.close_by)?,
                            true,
                        )
                        .map_err(|_| "history_unavailable")?,
                    );
                }
                let native = if let Some(o) = owned
                    .as_mut()
                    .filter(|o| o.permission_id == p.id && o.target == g.session)
                {
                    Some(&mut o.native)
                } else {
                    temporary.as_mut()
                };
                crate::reconcile_with_native(c, conn, &g, native, true)
            })();
            match result {
                Err("history_unavailable" | "expired") => {
                    // Preserve the last durable receipt without inventing a
                    // fresh native observation or bypassing revocation.
                    child_grant(
                        conn,
                        &p,
                        request.grant_handle.as_deref().ok_or("unauthorized")?,
                        &g.request,
                    )?;
                    let mut value = crate::checked_projection(conn, &g)?.ok_or("unauthorized")?;
                    value["reconciliation"] = json!("unavailable");
                    Ok(value)
                }
                other => other,
            }
        }
        _ => Err("unauthorized"),
    }
}

fn socket_path(c: &crate::wire::Config) -> std::path::PathBuf {
    Path::new(&c.lock_dir).join("provision-owner.sock")
}
fn read_line_bounded(
    stream: &mut std::os::unix::net::UnixStream,
    until: std::time::Instant,
) -> Result<Vec<u8>> {
    let mut b = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let left = until
            .checked_duration_since(std::time::Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or("bridge_unavailable")?;
        fail(stream.set_read_timeout(Some(left)))?;
        let n = fail(stream.read(&mut chunk))?;
        if n == 0 {
            return Err("bridge_unavailable");
        }
        b.extend_from_slice(&chunk[..n]);
        if b.len() > 256 * 1024 {
            return Err("bridge_unavailable");
        }
        if let Some(index) = b.iter().position(|c| *c == b'\n') {
            if index + 1 != b.len() {
                return Err("bridge_unavailable");
            }
            b.pop();
            return Ok(b);
        }
    }
}
fn write_bounded(
    stream: &mut std::os::unix::net::UnixStream,
    data: &[u8],
    until: std::time::Instant,
) -> Result<()> {
    let mut sent = 0;
    while sent < data.len() {
        let left = until
            .checked_duration_since(std::time::Instant::now())
            .filter(|d| !d.is_zero())
            .ok_or("bridge_unavailable")?;
        fail(stream.set_write_timeout(Some(left)))?;
        let n = fail(stream.write(&data[sent..]))?;
        if n == 0 {
            return Err("bridge_unavailable");
        }
        sent += n;
    }
    Ok(())
}
pub fn send(c: &crate::wire::Config, request: &OwnerRpc) -> Result<Value> {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut stream = fail(crate::native::connect_until(&socket_path(c), until))?;
    let mut data = fail(serde_json::to_vec(request))?;
    data.push(b'\n');
    write_bounded(&mut stream, &data, until)?;
    let value: Value = fail(serde_json::from_slice(&read_line_bounded(
        &mut stream,
        until,
    )?))?;
    if value.get("ok").and_then(Value::as_bool) == Some(true) {
        value.get("result").cloned().ok_or("bridge_unavailable")
    } else {
        Err(match value.get("error").and_then(Value::as_str) {
            Some("unauthorized") => "unauthorized",
            Some("expired") => "expired",
            Some("payload_conflict") => "payload_conflict",
            Some("target_mismatch") => "target_mismatch",
            Some("session_busy") => "session_busy",
            Some("history_unavailable") => "history_unavailable",
            Some("custody_lost") => "custody_lost",
            _ => "bridge_unavailable",
        })
    }
}
fn permission_by_id(conn: &Connection, id: &str) -> Result<Permission> {
    let (raw, expected, revoked): (String, String, i64) = fail(conn.query_row(
        "SELECT policy,policy_hash,revoked FROM provision_permissions WHERE permission_id=?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ))?;
    if wire::digest(raw.as_bytes()) != expected {
        return Err("bridge_unavailable");
    }
    Ok(Permission {
        id: id.to_owned(),
        policy: fail(serde_json::from_str(&raw))?,
        revoked: revoked != 0,
    })
}
pub fn serve(c: &crate::wire::Config, conn: &mut Connection) -> Result<Value> {
    check_schema(conn)?;
    use std::{
        fs::OpenOptions,
        io::{Seek, SeekFrom},
        os::unix::{
            fs::{FileTypeExt, MetadataExt, OpenOptionsExt},
            net::UnixListener,
        },
        time::Duration,
    };
    let lock_path = Path::new(&c.lock_dir).join("provision-owner.lock");
    let mut lock = fail(
        OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(&lock_path),
    )?;
    store::secure(&lock_path, false)?;
    fail(lock.try_lock())?;
    let path = socket_path(c);
    if let Ok(meta) = fs::symlink_metadata(&path) {
        if !meta.file_type().is_socket() || meta.uid() != unsafe { libc::geteuid() } {
            return Err("bridge_unavailable");
        }
        if fail(lock.metadata())?.len() > 128 {
            return Err("bridge_unavailable");
        }
        let mut marker = String::new();
        fail(lock.read_to_string(&mut marker))?;
        if marker != format!("{}:{}\n", meta.dev(), meta.ino()) {
            return Err("bridge_unavailable");
        }
        match crate::native::connect_until(
            &path,
            std::time::Instant::now() + Duration::from_secs(1),
        ) {
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => (),
            _ => return Err("bridge_unavailable"),
        }
        let current = fail(fs::symlink_metadata(&path))?;
        if current.dev() != meta.dev() || current.ino() != meta.ino() {
            return Err("bridge_unavailable");
        }
        fail(fs::remove_file(&path))?;
    }
    let listener = fail(UnixListener::bind(&path))?;
    let metadata = fail(fs::symlink_metadata(&path))?;
    fail(lock.seek(SeekFrom::Start(0)))?;
    fail(lock.set_len(0))?;
    fail(writeln!(lock, "{}:{}", metadata.dev(), metadata.ino()))?;
    fail(lock.sync_all())?;
    fail(fs::File::open(&c.lock_dir).and_then(|f| f.sync_all()))?;
    fail(listener.set_nonblocking(true))?;
    // A new owner cannot claim the old process's connection. Preserve every
    // allocation and make loss visible before serving further requests.
    fail(conn.execute("UPDATE provision_permissions SET setup_state=CASE WHEN setup_state='ready' THEN 'custody_lost' ELSE setup_state END,custody_generation=NULL WHERE setup_state!='closed'",[]))?;
    let mut owned: Option<Owned> = None;
    let mut reaped = std::collections::HashSet::new();
    let result = (|| -> Result<Value> {
        loop {
            // A restarted owner has no held connection, but still owns the
            // expiry obligation for known and uncertain reservations.
            let pending = {
                let mut query = fail(conn.prepare("SELECT permission_id FROM provision_permissions WHERE setup_state NOT IN ('unused','closed','refused_before_allocation') LIMIT 17"))?;
                let rows = fail(query.query_map([], |r| r.get::<_, String>(0)))?;
                fail(rows.collect::<std::result::Result<Vec<_>, _>>())?
            };
            if pending.len() > 16 {
                return Err("bridge_unavailable");
            }
            for id in pending {
                if reaped.contains(&id) {
                    continue;
                }
                let p = permission_by_id(conn, &id)?;
                if store::now() >= p.policy.close_by.saturating_sub(10) {
                    reaped.insert(id);
                    let _ = close(c, conn, &p, &mut owned);
                }
            }
            let (lost, activity) = match owned.as_mut() {
                Some(o) => match o.native.idle_poll() {
                    Ok(activity) => (None, activity),
                    Err(_) => (Some(o.permission_id.clone()), false),
                },
                None => (None, false),
            };
            if let Some(id) = lost {
                fail(conn.execute("UPDATE provision_permissions SET setup_state=CASE WHEN setup_state='ready' THEN 'custody_lost' ELSE setup_state END,custody_generation=NULL WHERE permission_id=?1",[&id]))?;
                owned = None;
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let until = std::time::Instant::now() + Duration::from_secs(1);
                    let result = read_line_bounded(&mut stream, until)
                        .and_then(|b| {
                            serde_json::from_slice::<OwnerRpc>(&b).map_err(|_| "unauthorized")
                        })
                        .and_then(|r| owner_call(c, conn, r, &mut owned));
                    let value = match result {
                        Ok(v) => json!({"ok":true,"result":v}),
                        Err(e) => json!({"ok":false,"error":e}),
                    };
                    if let Ok(mut data) = serde_json::to_vec(&value) {
                        data.push(b'\n');
                        let _ = write_bounded(
                            &mut stream,
                            &data,
                            std::time::Instant::now() + Duration::from_secs(1),
                        );
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if !activity {
                        std::thread::sleep(Duration::from_millis(50));
                    }
                }
                Err(_) => return Err("bridge_unavailable"),
            }
        }
    })();
    if fs::symlink_metadata(&path)
        .is_ok_and(|m| m.dev() == metadata.dev() && m.ino() == metadata.ino())
    {
        let _ = fs::remove_file(&path);
    }
    result
}
