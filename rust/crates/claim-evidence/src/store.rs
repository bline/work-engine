use crate::authority::{
    Bootstrap, BootstrapFiles, OpenFiles, PROFILE, checked_absolute_root, grant_digest, load,
    validate_grant,
};
use crate::codec::{
    JsValue, canonical_json_unchecked as canonical_json, digest_unchecked as digest, exact_fields,
    field, parse_json,
};
use crate::contract::{ClaimError, ClaimResult, RootIdentity, ensure};
use crate::findings::{
    array, legacy_text_field, revision_id, stable_claim_id, text_field, validate_revision_payload,
};
use rusqlite::{Connection, OpenFlags, params};
use std::{
    fs,
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
};

const MAX_STATE_BYTES: usize = 16 * 1024 * 1024;
const SCHEMA: &str = "native-review-claims-sqlite-v2";
const MARKER: &str = "claim-evidence-root.json";
const DATABASE: &str = "claim-evidence.sqlite3";

pub(crate) struct Store {
    pub conn: Connection,
    pub identity: RootIdentity,
    pub bootstrap: Bootstrap,
    files: OpenFiles,
    dev: u64,
    ino: u64,
    db_dev: u64,
    db_ino: u64,
}

fn io(error: impl std::fmt::Display) -> ClaimError {
    ClaimError::new(format!("claims storage: {error}"))
}
fn metadata(path: &Path) -> ClaimResult<(u64, u64)> {
    let m = fs::symlink_metadata(path).map_err(io)?;
    ensure(!m.file_type().is_symlink(), "claims path is symlink")?;
    Ok((m.dev(), m.ino()))
}
fn private_mode(path: &Path, mode: u32) -> ClaimResult<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(io)
}
fn db_open(path: &Path) -> ClaimResult<Connection> {
    let conn = Connection::open(path).map_err(io)?;
    conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA busy_timeout=5000; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;").map_err(io)?;
    private_mode(path, 0o600)?;
    harden_sidecars(path)?;
    Ok(conn)
}
pub(crate) fn harden_sidecars(path: &Path) -> ClaimResult<()> {
    for suffix in ["-wal", "-shm"] {
        let p = PathBuf::from(format!("{}{suffix}", path.display()));
        if p.exists() {
            private_mode(&p, 0o600)?;
        }
    }
    Ok(())
}
fn marker(identity: &RootIdentity, bootstrap: &Bootstrap, dev: u64, ino: u64) -> JsValue {
    JsValue::object([
        ("schema", JsValue::text(SCHEMA)),
        ("root_id", JsValue::text(&identity.root_id)),
        ("profile", JsValue::text(PROFILE)),
        (
            "path",
            JsValue::text(&identity.absolute_path.to_string_lossy()),
        ),
        ("config_sha256", JsValue::text(&bootstrap.config_sha256)),
        ("source_sha256", JsValue::text(&bootstrap.source_sha256)),
        ("directory_dev", JsValue::text(&dev.to_string())),
        ("directory_ino", JsValue::text(&ino.to_string())),
    ])
}
fn schema_fingerprint(conn: &Connection) -> ClaimResult<String> {
    let mut statement = conn
        .prepare("SELECT type,name,tbl_name,coalesce(sql,'') FROM sqlite_master ORDER BY type,name,tbl_name")
        .map_err(io)?;
    let rows = statement
        .query_map([], |row| {
            Ok(JsValue::Array(vec![
                JsValue::text(&row.get::<_, String>(0)?),
                JsValue::text(&row.get::<_, String>(1)?),
                JsValue::text(&row.get::<_, String>(2)?),
                JsValue::text(&row.get::<_, String>(3)?),
            ]))
        })
        .map_err(io)?;
    Ok(digest(&JsValue::Array(
        rows.collect::<Result<Vec<_>, _>>().map_err(io)?,
    )))
}
fn blank_state(grants: &[JsValue]) -> JsValue {
    JsValue::object([
        ("schema_version", JsValue::Number(1.0)),
        (
            "projection_boundary",
            JsValue::object([
                (
                    "actual_content_set",
                    JsValue::text("all records in canonical/store.json"),
                ),
                ("source_watermark", JsValue::Null),
                ("excluded_inputs", JsValue::Array(vec![])),
                ("failed_inputs", JsValue::Array(vec![])),
                ("freshness", JsValue::text("current_after_verified_rebuild")),
                ("completeness", JsValue::text("available")),
            ]),
        ),
        ("authorities", JsValue::Array(grants.to_vec())),
        ("claims", JsValue::Array(vec![])),
        ("revisions", JsValue::Array(vec![])),
        ("lineage", JsValue::Array(vec![])),
        ("reliances", JsValue::Array(vec![])),
        ("operations", JsValue::Array(vec![])),
    ])
}
impl Store {
    pub fn initialize(files: BootstrapFiles) -> ClaimResult<Self> {
        let open: OpenFiles = files.into();
        let bootstrap = load(&open)?;
        ensure(open.root.is_absolute(), "root must be absolute")?;
        let parent = open
            .root
            .parent()
            .ok_or_else(|| ClaimError::new("root parent absent"))?;
        ensure(
            parent.canonicalize().map_err(io)? == parent,
            "root parent is redirected",
        )?;
        if open.root.exists() {
            ensure(
                !fs::symlink_metadata(&open.root)
                    .map_err(io)?
                    .file_type()
                    .is_symlink(),
                "root is symlink",
            )?;
            ensure(
                open.root.read_dir().map_err(io)?.next().is_none(),
                "root is not empty",
            )?;
        } else {
            fs::create_dir(&open.root).map_err(io)?;
        }
        private_mode(&open.root, 0o700)?;
        let absolute = checked_absolute_root(&open.root)?;
        let (dev, ino) = metadata(&absolute)?;
        let identity = RootIdentity {
            absolute_path: absolute.clone(),
            root_id: bootstrap.root_id.clone(),
            profile: PROFILE.into(),
        };
        let marker_json = canonical_json(&marker(&identity, &bootstrap, dev, ino));
        let marker_path = absolute.join(MARKER);
        fs::write(&marker_path, &marker_json).map_err(io)?;
        private_mode(&marker_path, 0o600)?;
        fs::File::open(&marker_path)
            .and_then(|f| f.sync_all())
            .map_err(io)?;
        fs::File::open(&absolute)
            .and_then(|f| f.sync_all())
            .map_err(io)?;
        let conn = db_open(&absolute.join(DATABASE))?;
        let (db_dev, db_ino) = metadata(&absolute.join(DATABASE))?;
        conn.execute_batch(include_str!(
            "../migrations/0002_private_production_path.sql"
        ))
        .map_err(io)?;
        let schema_sha256 = schema_fingerprint(&conn)?;
        conn.execute(
            "INSERT INTO root_binding VALUES(1,?,?,?,?)",
            params![
                crate::authority::sha256_bytes(marker_json.as_bytes()),
                db_dev.to_string(),
                db_ino.to_string(),
                schema_sha256
            ],
        )
        .map_err(io)?;
        let state = blank_state(&bootstrap.grants);
        let json = canonical_json(&state);
        conn.execute(
            "INSERT INTO canonical_claim_state VALUES(1,1,?,?)",
            params![digest(&state), json],
        )
        .map_err(io)?;
        crate::fault_cut("before_root_commit");
        conn.execute_batch("COMMIT").map_err(io)?;
        crate::fault_cut("after_root_commit");
        fs::File::open(&absolute)
            .and_then(|f| f.sync_all())
            .map_err(io)?;
        let store = Self {
            conn,
            identity,
            bootstrap,
            files: open,
            dev,
            ino,
            db_dev,
            db_ino,
        };
        store.check_root()?;
        store.read_state()?;
        Ok(store)
    }

    pub fn open(files: OpenFiles) -> ClaimResult<Self> {
        let bootstrap = load(&files)?;
        let absolute = checked_absolute_root(&files.root)?;
        let (dev, ino) = metadata(&absolute)?;
        let identity = RootIdentity {
            absolute_path: absolute.clone(),
            root_id: bootstrap.root_id.clone(),
            profile: PROFILE.into(),
        };
        let db_path = absolute.join(DATABASE);
        ensure(db_path.exists(), "claims database absent")?;
        let (db_dev, db_ino) = metadata(&db_path)?;
        let expected_marker = canonical_json(&marker(&identity, &bootstrap, dev, ino));
        ensure(
            fs::read(absolute.join(MARKER)).map_err(io)? == expected_marker.as_bytes(),
            "root marker mismatch",
        )?;
        let read_only =
            Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(io)?;
        read_only
            .busy_timeout(std::time::Duration::from_millis(5000))
            .map_err(io)?;
        let preliminary = Self {
            conn: read_only,
            identity: identity.clone(),
            bootstrap: bootstrap.clone(),
            files: files.clone(),
            dev,
            ino,
            db_dev,
            db_ino,
        };
        preliminary.check_root()?;
        preliminary.read_state()?;
        drop(preliminary);
        ensure(
            metadata(&db_path)? == (db_dev, db_ino),
            "database path changed during open",
        )?;
        let conn = db_open(&db_path)?;
        let store = Self {
            conn,
            identity,
            bootstrap,
            files,
            dev,
            ino,
            db_dev,
            db_ino,
        };
        store.check_root()?;
        store.read_state()?;
        Ok(store)
    }

    pub fn check_root(&self) -> ClaimResult<()> {
        let current = load(&self.files)?;
        ensure(
            current.root_id == self.bootstrap.root_id
                && current.config_sha256 == self.bootstrap.config_sha256
                && current.source_sha256 == self.bootstrap.source_sha256
                && current.grants == self.bootstrap.grants
                && current.trusted_custody == self.bootstrap.trusted_custody,
            "bootstrap files changed",
        )?;
        let actual = checked_absolute_root(&self.identity.absolute_path)?;
        ensure(actual == self.identity.absolute_path, "root path changed")?;
        ensure(
            metadata(&actual)? == (self.dev, self.ino),
            "root directory changed",
        )?;
        ensure(
            metadata(&actual.join(DATABASE))? == (self.db_dev, self.db_ino),
            "database path changed",
        )?;
        let marker_bytes = fs::read(actual.join(MARKER)).map_err(io)?;
        let expected = canonical_json(&marker(&self.identity, &self.bootstrap, self.dev, self.ino));
        ensure(marker_bytes == expected.as_bytes(), "root marker mismatch")?;
        let schema: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(io)?;
        ensure(schema == 2, "claims schema version mismatch")?;
        let migration: String = self
            .conn
            .query_row(
                "SELECT name FROM schema_migrations WHERE version=2",
                [],
                |r| r.get(0),
            )
            .map_err(io)?;
        ensure(migration == SCHEMA, "claims migration history mismatch")?;
        let migration_count: i64 = self
            .conn
            .query_row("SELECT count(*) FROM schema_migrations", [], |r| r.get(0))
            .map_err(io)?;
        ensure(
            migration_count == 1,
            "claims migration history has extra rows",
        )?;
        let (hash,dev,ino,schema_sha256):(String,String,String,String)=self.conn.query_row("SELECT marker_sha256,database_dev,database_ino,schema_sha256 FROM root_binding WHERE singleton=1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(io)?;
        ensure(
            hash == crate::authority::sha256_bytes(&marker_bytes)
                && dev == self.db_dev.to_string()
                && ino == self.db_ino.to_string()
                && schema_sha256 == schema_fingerprint(&self.conn)?,
            "database root binding mismatch",
        )?;
        let integrity: String = self
            .conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .map_err(io)?;
        ensure(integrity == "ok", "SQLite integrity failure")?;
        Ok(())
    }

    pub fn read_state(&self) -> ClaimResult<(i64, JsValue)> {
        self.check_root()?;
        let (revision,sha,json):(i64,String,String)=self.conn.query_row("SELECT store_revision,store_sha256,store_json FROM canonical_claim_state WHERE singleton=1",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(io)?;
        ensure(
            revision >= 1 && json.len() <= MAX_STATE_BYTES,
            "canonical state capacity invalid",
        )?;
        let state = parse_json(&json)?;
        ensure(
            canonical_json(&state) == json && digest(&state) == sha,
            "canonical state integrity failure",
        )?;
        validate_state(&state, &self.bootstrap.grants)?;
        let count: i64 = self
            .conn
            .query_row("SELECT count(*) FROM operation_admissions", [], |r| {
                r.get(0)
            })
            .map_err(io)?;
        ensure(
            count == records(&state, "operations")?.len() as i64,
            "operation admission count mismatch",
        )?;
        for operation in records(&state, "operations")? {
            let id = text_field(operation, "operation_id")?;
            let binding:(String,String)=self.conn.query_row("SELECT admission_sha256,grant_sha256 FROM operation_admissions WHERE operation_id=?",[id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(io)?;
            let grant = self
                .bootstrap
                .grants
                .iter()
                .find(|g| {
                    text_field(g, "grant_id").ok() == text_field(operation, "authority_ref").ok()
                })
                .ok_or_else(|| ClaimError::new("operation grant absent"))?;
            ensure(
                binding.1 == grant_digest(grant)
                    && binding.0.len() == 64
                    && binding
                        .0
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
                "operation admission binding mismatch",
            )?;
        }
        Ok((revision, state))
    }
}

pub(crate) fn records<'a>(state: &'a JsValue, key: &str) -> ClaimResult<&'a [JsValue]> {
    array(field(state, key)?, key)
}
pub(crate) fn records_mut<'a>(
    state: &'a mut JsValue,
    key: &str,
) -> ClaimResult<&'a mut Vec<JsValue>> {
    match state {
        JsValue::Object(map) => match map.get_mut(&crate::codec::JsString::new(key)) {
            Some(JsValue::Array(items)) => Ok(items),
            _ => Err(ClaimError::new("invalid state collection")),
        },
        _ => Err(ClaimError::new("invalid state")),
    }
}
pub(crate) fn state_json(state: &JsValue) -> ClaimResult<String> {
    let json = canonical_json(state);
    ensure(
        json.len() <= MAX_STATE_BYTES,
        "canonical state exceeds capacity",
    )?;
    // Reject a state the normal reader could not parse after commit.
    ensure(
        crate::codec::canonical_json(state)? == json,
        "state codec mismatch",
    )?;
    Ok(json)
}

pub(crate) fn validate_state(state: &JsValue, grants: &[JsValue]) -> ClaimResult<()> {
    exact_fields(
        state,
        &[
            "schema_version",
            "projection_boundary",
            "authorities",
            "claims",
            "revisions",
            "lineage",
            "reliances",
            "operations",
        ],
        "state",
    )?;
    ensure(
        field(state, "schema_version")? == &JsValue::Number(1.0),
        "state schema mismatch",
    )?;
    let boundary = field(state, "projection_boundary")?;
    exact_fields(
        boundary,
        &[
            "actual_content_set",
            "source_watermark",
            "excluded_inputs",
            "failed_inputs",
            "freshness",
            "completeness",
        ],
        "projection boundary",
    )?;
    ensure(
        text_field(boundary, "actual_content_set")? == "all records in canonical/store.json"
            && text_field(boundary, "freshness")? == "current_after_verified_rebuild"
            && text_field(boundary, "completeness")? == "available"
            && field(boundary, "source_watermark")? == &JsValue::Null
            && array(field(boundary, "excluded_inputs")?, "excluded_inputs")?.is_empty()
            && array(field(boundary, "failed_inputs")?, "failed_inputs")?.is_empty(),
        "unsupported projection boundary",
    )?;
    ensure(
        records(state, "authorities")? == grants,
        "registered grant set changed",
    )?;
    ensure(
        records(state, "lineage")?.is_empty(),
        "unsupported lineage present",
    )?;
    let mut ids = std::collections::HashSet::new();
    let mut claims = std::collections::HashMap::new();
    for claim in records(state, "claims")? {
        exact_fields(
            claim,
            &[
                "id",
                "schema_version",
                "profile",
                "subject",
                "statement_identity",
                "created_by",
                "authority_ref",
            ],
            "claim",
        )?;
        let id = text_field(claim, "id")?;
        ensure(
            field(claim, "schema_version")? == &JsValue::Number(1.0),
            "claim schema mismatch",
        )?;
        ensure(
            id == stable_claim_id(field(claim, "subject")?)?.as_str(),
            "claim identity mismatch",
        )?;
        ensure(ids.insert(id.clone()), "duplicate claim identity")?;
        ensure(
            text_field(claim, "profile")? == crate::authority::FINDING_PROFILE,
            "claim profile mismatch",
        )?;
        let authority_ref = text_field(claim, "authority_ref")?;
        let grant = grants
            .iter()
            .find(|g| text_field(g, "grant_id").ok().as_deref() == Some(authority_ref.as_str()))
            .ok_or_else(|| ClaimError::new("claim authority absent"))?;
        ensure(
            text_field(claim, "created_by")? == text_field(grant, "actor")?,
            "claim actor mismatch",
        )?;
        ensure(
            legacy_text_field(claim, "statement_identity").is_ok(),
            "claim statement absent",
        )?;
        ensure(
            array(field(grant, "permissions")?, "permissions")?
                .contains(&JsValue::text("create_claim")),
            "claim grant lacks create permission",
        )?;
        claims.insert(id, claim);
    }
    let mut revisions = std::collections::HashMap::new();
    for rev in records(state, "revisions")? {
        let mut map = rev.as_object()?.clone();
        map.remove(&crate::codec::JsString::new("id"));
        let id = text_field(rev, "id")?;
        ensure(
            field(rev, "schema_version")? == &JsValue::Number(1.0),
            "revision schema mismatch",
        )?;
        ensure(
            id == revision_id(&JsValue::Object(map))?.as_str(),
            "revision identity mismatch",
        )?;
        ensure(ids.insert(id.clone()), "duplicate revision identity")?;
        let claim_id = text_field(rev, "claim_id")?;
        ensure(claims.contains_key(&claim_id), "revision claim absent")?;
        let authority_ref = text_field(rev, "authority_ref")?;
        let grant = grants
            .iter()
            .find(|g| text_field(g, "grant_id").ok().as_deref() == Some(authority_ref.as_str()))
            .ok_or_else(|| ClaimError::new("revision authority absent"))?;
        ensure(
            text_field(rev, "producer")? == text_field(grant, "actor")?,
            "revision producer mismatch",
        )?;
        ensure(
            text_field(grant, "profile")? == crate::authority::FINDING_PROFILE,
            "revision grant profile mismatch",
        )?;
        let mut payload = rev.as_object()?.clone();
        for key in [
            "id",
            "schema_version",
            "claim_id",
            "predecessor_revision",
            "producer",
            "authority_ref",
        ] {
            payload.remove(&crate::codec::JsString::new(key));
        }
        validate_revision_payload(
            &JsValue::Object(payload),
            &text_field(grant, "decision_scope")?,
        )?;
        revisions.insert(id, rev);
    }
    for rev in records(state, "revisions")? {
        if let JsValue::String(_) = field(rev, "predecessor_revision")? {
            let predecessor = text_field(rev, "predecessor_revision")?;
            ensure(
                revisions.get(&predecessor).is_some_and(|p| {
                    text_field(p, "claim_id").ok()
                        == Some(text_field(rev, "claim_id").unwrap_or_default())
                }),
                "revision predecessor absent or crossed",
            )?;
        } else {
            ensure(
                field(rev, "predecessor_revision")? == &JsValue::Null,
                "invalid revision predecessor",
            )?;
        }
    }
    for grant in grants {
        validate_grant(grant)?;
    }
    let mut reliances = std::collections::HashMap::new();
    for reliance in records(state, "reliances")? {
        exact_fields(
            reliance,
            &[
                "id",
                "schema_version",
                "consumer",
                "consumer_revision",
                "decision_scope",
                "claim_revision_id",
                "state",
                "predecessor_reliance",
                "authority_ref",
                "operation_id",
            ],
            "reliance",
        )?;
        let id = text_field(reliance, "id")?;
        ensure(
            field(reliance, "schema_version")? == &JsValue::Number(1.0)
                && field(reliance, "state")? == &JsValue::text("active")
                && field(reliance, "predecessor_reliance")? == &JsValue::Null,
            "unsupported reliance state",
        )?;
        for key in [
            "consumer",
            "consumer_revision",
            "decision_scope",
            "claim_revision_id",
            "authority_ref",
            "operation_id",
        ] {
            text_field(reliance, key)?;
        }
        let mut without_id = reliance.as_object()?.clone();
        without_id.remove(&crate::codec::JsString::new("id"));
        ensure(
            id == format!("reliance@{}", digest(&JsValue::Object(without_id))),
            "reliance identity mismatch",
        )?;
        ensure(ids.insert(id.clone()), "duplicate reliance identity")?;
        let target = revisions
            .get(&text_field(reliance, "claim_revision_id")?)
            .ok_or_else(|| ClaimError::new("reliance target absent"))?;
        let grant = grants
            .iter()
            .find(|g| text_field(g, "grant_id").ok() == text_field(reliance, "authority_ref").ok())
            .ok_or_else(|| ClaimError::new("reliance grant absent"))?;
        let claim = claims
            .get(&text_field(target, "claim_id")?)
            .ok_or_else(|| ClaimError::new("reliance claim absent"))?;
        ensure(
            text_field(grant, "profile")? == text_field(claim, "profile")?
                && text_field(grant, "decision_scope")? == text_field(reliance, "decision_scope")?
                && text_field(target, "decision_scope")? == text_field(reliance, "decision_scope")?
                && array(field(grant, "permissions")?, "permissions")?
                    .contains(&JsValue::text("record_reliance")),
            "reliance authority mismatch",
        )?;
        reliances.insert(id, reliance);
    }
    let mut ops = std::collections::HashSet::new();
    for op in records(state, "operations")? {
        exact_fields(
            op,
            &[
                "operation_id",
                "action",
                "payload_sha256",
                "result_identity",
                "authority_ref",
            ],
            "operation receipt",
        )?;
        let id = text_field(op, "operation_id")?;
        ensure(
            ops.insert(id.clone()) && ids.insert(id),
            "duplicate operation identity",
        )?;
        let action = text_field(op, "action")?;
        ensure(
            ["create_claim", "publish_revision", "record_reliance"].contains(&action.as_str()),
            "unsupported operation receipt",
        )?;
        let authority_ref = text_field(op, "authority_ref")?;
        let grant = grants
            .iter()
            .find(|g| text_field(g, "grant_id").ok().as_deref() == Some(authority_ref.as_str()))
            .ok_or_else(|| ClaimError::new("receipt grant absent"))?;
        ensure(
            array(field(grant, "permissions")?, "permissions")?.contains(&JsValue::text(&action)),
            "receipt grant permission mismatch",
        )?;
        let result = if action == "record_reliance" {
            reliances.get(&text_field(op, "result_identity")?)
        } else {
            revisions.get(&text_field(op, "result_identity")?)
        }
        .ok_or_else(|| ClaimError::new("receipt result absent"))?;
        ensure(
            text_field(result, "authority_ref")? == authority_ref,
            "operation result authority mismatch",
        )?;
        if action == "record_reliance" {
            ensure(
                text_field(result, "operation_id")? == text_field(op, "operation_id")?,
                "reliance receipt operation mismatch",
            )?;
        } else {
            ensure(
                (action == "create_claim")
                    == (field(result, "predecessor_revision")? == &JsValue::Null),
                "operation result action mismatch",
            )?;
        }
        let sha = text_field(op, "payload_sha256")?;
        ensure(
            sha.len() == 64
                && sha
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
            "operation digest invalid",
        )?;
        if action == "record_reliance" {
            ensure(
                sha == reliance_operation_digest(result, grant)?,
                "reliance operation payload mismatch",
            )?;
        }
    }
    for reliance in records(state, "reliances")? {
        ensure(
            records(state, "operations")?.iter().any(|op| {
                text_field(op, "action").ok().as_deref() == Some("record_reliance")
                    && text_field(op, "operation_id").ok()
                        == text_field(reliance, "operation_id").ok()
                    && text_field(op, "result_identity").ok() == text_field(reliance, "id").ok()
            }),
            "reliance operation receipt absent",
        )?;
    }
    Ok(())
}

fn reliance_operation_digest(reliance: &JsValue, grant: &JsValue) -> ClaimResult<String> {
    let payload = JsValue::object([
        ("consumer", field(reliance, "consumer")?.clone()),
        (
            "consumer_revision",
            field(reliance, "consumer_revision")?.clone(),
        ),
        ("decision_scope", field(reliance, "decision_scope")?.clone()),
        (
            "claim_revision_id",
            field(reliance, "claim_revision_id")?.clone(),
        ),
        ("state", field(reliance, "state")?.clone()),
        (
            "predecessor_reliance",
            field(reliance, "predecessor_reliance")?.clone(),
        ),
    ]);
    let operation = JsValue::object([
        ("schema_version", JsValue::Number(1.0)),
        ("operation_id", field(reliance, "operation_id")?.clone()),
        ("action", JsValue::text("record_reliance")),
        ("profile", field(grant, "profile")?.clone()),
        ("expected_state", JsValue::Null),
        ("payload", payload),
    ]);
    Ok(digest(&JsValue::object([
        ("operation", operation),
        ("admitted_grant", grant.clone()),
    ])))
}

#[cfg(test)]
mod tests {
    use super::state_json;
    use crate::codec::JsValue;

    #[test]
    fn final_state_must_be_readable_by_the_same_codec() {
        let mut deep = JsValue::Null;
        for _ in 0..257 {
            deep = JsValue::Array(vec![deep]);
        }
        assert!(state_json(&deep).is_err());
    }
}
