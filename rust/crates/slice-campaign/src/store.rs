use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

use crate::codec::{campaign_digest, campaign_update_revision_digest, raw_sha256};
use crate::contract::{CampaignIdentity, CampaignRevision, Snapshot};
use crate::recovery::{OperationReceipt, Reconcile, RecoveryLocator};
use crate::{CampaignError, Result};

const MARKER: &str = ".campaign-native-initial-v2";
const DATABASE: &str = "slice-campaign.sqlite";
const LOCK: &str = ".writer.lock";
const SCHEMA: &str = include_str!("../migrations/0001_private_review.sql");
const SCHEMA_2: &str = include_str!("../migrations/0002_initial_completion.sql");

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    schema_version: u32,
    profile: String,
    root_id: String,
    anchored_root_sha256: String,
    trusted_config_digest: String,
    writer_identity: String,
}

pub(crate) struct Store {
    pub(crate) root: PathBuf,
    pub(crate) root_id: String,
    pub(crate) conn: Connection,
    pub(crate) epoch: String,
    marker: Marker,
    root_dev: u64,
    root_ino: u64,
    database_dev: u64,
    database_ino: u64,
    _lock: File,
}

pub(crate) struct SlotHistory {
    pub(crate) obligation_id: String,
    pub(crate) attempt_id: String,
    pub(crate) request_digest: String,
    pub(crate) prepared_revision: String,
    pub(crate) dispatch_operation_id: Option<String>,
    pub(crate) may_have_entered: bool,
    pub(crate) active: bool,
}

fn root_path_digest(root: &Path) -> String {
    raw_sha256(root.as_os_str().as_bytes())
}

fn random_id() -> Result<String> {
    let mut bytes = [0u8; 16];
    File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut bytes))
        .map_err(|e| CampaignError::Io(e.to_string()))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn anchored_parent(root: &Path) -> Result<PathBuf> {
    if !root.is_absolute() {
        return Err(CampaignError::Root("root must be absolute".into()));
    }
    let parent = root
        .parent()
        .ok_or_else(|| CampaignError::Root("root has no parent".into()))?;
    let canonical = fs::canonicalize(parent).map_err(|e| CampaignError::Root(e.to_string()))?;
    if canonical != parent {
        return Err(CampaignError::Root(
            "root parent has symlink or alias".into(),
        ));
    }
    Ok(canonical)
}

fn regular_nofollow(path: &Path) -> Result<File> {
    let metadata = fs::symlink_metadata(path).map_err(|e| CampaignError::Root(e.to_string()))?;
    if !metadata.file_type().is_file() || metadata.permissions().mode() & 0o077 != 0 {
        return Err(CampaignError::Root(format!(
            "unsafe private file {}",
            path.display()
        )));
    }
    OpenOptions::new()
        .read(true)
        .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
        .open(path)
        .map_err(|e| CampaignError::Root(e.to_string()))
}

fn verify_sidecars(root: &Path) -> Result<()> {
    for name in [
        "slice-campaign.sqlite-wal",
        "slice-campaign.sqlite-shm",
        "slice-campaign.sqlite-journal",
    ] {
        let path = root.join(name);
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_file() && meta.permissions().mode() & 0o077 == 0 => {}
            Ok(_) => {
                return Err(CampaignError::Root(format!(
                    "unsafe SQLite sidecar {}",
                    path.display()
                )));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(CampaignError::Root(error.to_string())),
        }
    }
    Ok(())
}

impl Store {
    pub(crate) fn initialize(
        root: &Path,
        config_digest: &str,
        writer_identity: &str,
    ) -> Result<Self> {
        anchored_parent(root)?;
        fs::create_dir(root).map_err(|e| CampaignError::Root(e.to_string()))?;
        fs::set_permissions(root, fs::Permissions::from_mode(0o700))
            .map_err(|e| CampaignError::Root(e.to_string()))?;
        let marker = Marker {
            schema_version: 2,
            profile: crate::contract::PROFILE.into(),
            root_id: random_id()?,
            anchored_root_sha256: root_path_digest(root),
            trusted_config_digest: config_digest.into(),
            writer_identity: writer_identity.into(),
        };
        let bytes = serde_json::to_vec(&marker).map_err(|e| CampaignError::Io(e.to_string()))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(root.join(MARKER))
            .map_err(|e| CampaignError::Root(e.to_string()))?;
        use std::io::Write;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| CampaignError::Io(e.to_string()))?;
        let conn =
            Connection::open(root.join(DATABASE)).map_err(|e| CampaignError::Io(e.to_string()))?;
        fs::set_permissions(root.join(DATABASE), fs::Permissions::from_mode(0o600))
            .map_err(|e| CampaignError::Io(e.to_string()))?;
        configure(&conn)?;
        conn.execute_batch(SCHEMA)
            .map_err(|e| CampaignError::Io(e.to_string()))?;
        conn.execute_batch(SCHEMA_2)
            .map_err(|e| CampaignError::Io(e.to_string()))?;
        conn.execute(
            "INSERT INTO store_identity(singleton,root_id,anchored_root_sha256,profile,trusted_config_digest,writer_identity) VALUES(1,?1,?2,?3,?4,?5)",
            params![marker.root_id, marker.anchored_root_sha256, marker.profile, marker.trusted_config_digest, marker.writer_identity],
        ).map_err(|e| CampaignError::Io(e.to_string()))?;
        conn.execute_batch("PRAGMA user_version = 2")
            .map_err(|e| CampaignError::Io(e.to_string()))?;
        drop(conn);
        File::open(root)
            .and_then(|directory| directory.sync_all())
            .map_err(|e| CampaignError::Io(e.to_string()))?;
        Self::open(root, config_digest, writer_identity)
    }

    pub(crate) fn open(root: &Path, config_digest: &str, writer_identity: &str) -> Result<Self> {
        anchored_parent(root)?;
        let metadata =
            fs::symlink_metadata(root).map_err(|e| CampaignError::Root(e.to_string()))?;
        if !metadata.file_type().is_dir()
            || metadata.permissions().mode() & 0o077 != 0
            || fs::canonicalize(root).ok().as_deref() != Some(root)
        {
            return Err(CampaignError::Root(
                "root is not an anchored private directory".into(),
            ));
        }
        let marker_file = regular_nofollow(&root.join(MARKER))?;
        let marker: Marker =
            serde_json::from_reader(marker_file).map_err(|e| CampaignError::Root(e.to_string()))?;
        if marker.schema_version != 2
            || marker.profile != crate::contract::PROFILE
            || marker.trusted_config_digest != config_digest
            || marker.writer_identity != writer_identity
            || marker.anchored_root_sha256 != root_path_digest(root)
        {
            return Err(CampaignError::Root(
                "root marker identity or profile differs".into(),
            ));
        }
        if root.join(LOCK).exists() {
            regular_nofollow(&root.join(LOCK))?;
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32)
            .open(root.join(LOCK))
            .map_err(|e| CampaignError::Root(e.to_string()))?;
        if lock
            .metadata()
            .map_err(|e| CampaignError::Root(e.to_string()))?
            .permissions()
            .mode()
            & 0o077
            != 0
        {
            return Err(CampaignError::Root("writer lock permissions differ".into()));
        }
        lock.try_lock()
            .map_err(|_| CampaignError::Root("another campaign writer owns root".into()))?;
        let held_database = regular_nofollow(&root.join(DATABASE))?;
        verify_sidecars(root)?;
        let before = held_database
            .metadata()
            .map_err(|e| CampaignError::Root(e.to_string()))?;
        // Refuse an unsupported schema before a writable connection can change
        // its journal mode or create SQLite sidecars.
        let probe = Connection::open_with_flags(
            root.join(DATABASE),
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| CampaignError::Root(e.to_string()))?;
        let version: i64 = probe
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .map_err(|e| CampaignError::Root(e.to_string()))?;
        drop(probe);
        if version != 2 {
            return Err(CampaignError::Root(
                "unsupported campaign store schema".into(),
            ));
        }
        let conn = Connection::open_with_flags(
            root.join(DATABASE),
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|e| CampaignError::Root(e.to_string()))?;
        let after = fs::symlink_metadata(root.join(DATABASE))
            .map_err(|e| CampaignError::Root(e.to_string()))?;
        use std::os::unix::fs::MetadataExt;
        if before.ino() != after.ino() || before.dev() != after.dev() {
            return Err(CampaignError::Root("database changed while opening".into()));
        }
        if conn
            .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
            .map_err(|e| CampaignError::Io(e.to_string()))?
            != 2
        {
            return Err(CampaignError::Root(
                "unsupported campaign store schema".into(),
            ));
        }
        configure(&conn)?;
        verify_sidecars(root)?;
        let store = Self {
            root: root.to_path_buf(),
            root_id: marker.root_id.clone(),
            conn,
            epoch: random_id()?,
            marker,
            root_dev: metadata.dev(),
            root_ino: metadata.ino(),
            database_dev: before.dev(),
            database_ino: before.ino(),
            _lock: lock,
        };
        store.verify_binding()?;
        Ok(store)
    }

    fn verify_binding(&self) -> Result<()> {
        let root_metadata = fs::symlink_metadata(&self.root)
            .map_err(|e| CampaignError::Integrity(e.to_string()))?;
        use std::os::unix::fs::MetadataExt;
        if !root_metadata.file_type().is_dir()
            || root_metadata.dev() != self.root_dev
            || root_metadata.ino() != self.root_ino
            || root_metadata.permissions().mode() & 0o077 != 0
            || fs::canonicalize(&self.root).ok().as_deref() != Some(self.root.as_path())
        {
            return Err(CampaignError::Integrity("anchored root changed".into()));
        }
        let metadata = fs::symlink_metadata(self.root.join(DATABASE))
            .map_err(|e| CampaignError::Integrity(e.to_string()))?;
        if !metadata.file_type().is_file()
            || metadata.dev() != self.database_dev
            || metadata.ino() != self.database_ino
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(CampaignError::Integrity(
                "database path or inode differs".into(),
            ));
        }
        let marker_file = regular_nofollow(&self.root.join(MARKER))
            .map_err(|e| CampaignError::Integrity(e.to_string()))?;
        let marker: Marker = serde_json::from_reader(marker_file)
            .map_err(|e| CampaignError::Integrity(e.to_string()))?;
        if marker.schema_version != self.marker.schema_version
            || marker.profile != self.marker.profile
            || marker.root_id != self.marker.root_id
            || marker.anchored_root_sha256 != self.marker.anchored_root_sha256
            || marker.trusted_config_digest != self.marker.trusted_config_digest
            || marker.writer_identity != self.marker.writer_identity
            || marker.anchored_root_sha256 != root_path_digest(&self.root)
        {
            return Err(CampaignError::Integrity("root marker changed".into()));
        }
        let database_identity: (String, String, String, String, String) = self.conn.query_row(
            "SELECT root_id,anchored_root_sha256,profile,trusted_config_digest,writer_identity FROM store_identity WHERE singleton=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).map_err(|e| CampaignError::Integrity(format!("database identity absent: {e}")))?;
        if database_identity
            != (
                marker.root_id,
                marker.anchored_root_sha256,
                marker.profile,
                marker.trusted_config_digest,
                marker.writer_identity,
            )
        {
            return Err(CampaignError::Integrity(
                "database root identity differs".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn read(&self, identity: &CampaignIdentity) -> Result<Option<Snapshot>> {
        self.verify_binding()?;
        let row: Option<(String, String)> = self
            .conn
            .query_row(
                "SELECT revision,state_json FROM campaign_state WHERE identity_key=?1",
                [identity.key()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| CampaignError::Io(e.to_string()))?;
        row.map(|(sql_revision, json)| {
            let state = self.decode_state(&json, identity)?;
            if state.revision.as_str() != sql_revision {
                return Err(CampaignError::Integrity(
                    "SQL campaign revision differs".into(),
                ));
            }
            let committed: Option<String> = self.conn.query_row(
                "SELECT operation_id FROM operation_receipt WHERE identity_key=?1 AND result_json=?2 LIMIT 1",
                params![identity.key(), json], |row| row.get(0),
            ).optional().map_err(|e| CampaignError::Integrity(e.to_string()))?;
            let operation_id = committed.ok_or_else(|| CampaignError::Integrity("current state lacks exact commit receipt".into()))?;
            self.replay_result(&operation_id)?.ok_or_else(|| CampaignError::Integrity("current commit receipt absent".into()))?;
            self.validate_progress(&state, false)?;
            Ok(state)
        })
        .transpose()
    }

    pub(crate) fn receipt(&self, operation_id: &str) -> Result<Option<OperationReceipt>> {
        Ok(self
            .replay_result(operation_id)?
            .map(|(receipt, _)| receipt))
    }

    pub(crate) fn replay_result(
        &self,
        operation_id: &str,
    ) -> Result<Option<(OperationReceipt, Snapshot)>> {
        self.verify_binding()?;
        let row: Option<(String, String, String, String, String)> = self
            .conn
            .query_row(
                "SELECT identity_key,kind,request_digest,receipt_json,result_json FROM operation_receipt WHERE operation_id=?1",
                [operation_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()
            .map_err(|e| CampaignError::Io(e.to_string()))?;
        row.map(|(identity_key, kind, request_digest, receipt, state)| {
            let receipt: OperationReceipt = serde_json::from_str(&receipt)
                .map_err(|e| CampaignError::Integrity(e.to_string()))?;
            let state: Snapshot = self.decode_state(&state, &receipt.identity)?;
            if receipt.root_id != self.root_id
                || receipt.operation_id != operation_id
                || receipt.kind != kind
                || receipt.request_digest != request_digest
                || receipt.identity.key() != identity_key
                || state.identity != receipt.identity
                || state.schema_version != 2
                || state.revision != receipt.result_revision
                || receipt.profile_digest != self.marker.trusted_config_digest
                || receipt.result_reference != format!("campaign:{}@{}", identity_key, state.revision.as_str())
                || !valid_sha(&receipt.request_digest)
            {
                return Err(CampaignError::Integrity(
                    "stored operation receipt/result differs".into(),
                ));
            }
            if receipt.kind == "admit" {
                if receipt.prior_revision.is_some() { return Err(CampaignError::Integrity("admission prior revision differs".into())); }
            } else {
                let prior = receipt.prior_revision.as_ref().ok_or_else(|| CampaignError::Integrity("operation prior revision absent".into()))?;
                let exists: Option<i64> = self.conn.query_row(
                    "SELECT 1 FROM operation_receipt WHERE identity_key=?1 AND json_extract(receipt_json,'$.resultRevision')=?2 LIMIT 1",
                    params![identity_key, prior.as_str()], |row| row.get(0),
                ).optional().map_err(|e| CampaignError::Integrity(e.to_string()))?;
                if exists.is_none() { return Err(CampaignError::Integrity("operation prior revision uncommitted".into())); }
            }
            self.validate_progress(&state, true)?;
            Ok((receipt, state))
        })
        .transpose()
    }

    fn decode_state(&self, json: &str, identity: &CampaignIdentity) -> Result<Snapshot> {
        if json.len() > 1_048_576 {
            return Err(CampaignError::Integrity(
                "stored campaign exceeds capacity".into(),
            ));
        }
        let state: Snapshot =
            serde_json::from_str(json).map_err(|e| CampaignError::Integrity(e.to_string()))?;
        if state.identity != *identity
            || state.schema_version != 2
            || state_revision(&state)? != state.revision
        {
            return Err(CampaignError::Integrity(
                "stored campaign identity, schema or revision differs".into(),
            ));
        }
        Ok(state)
    }

    /// Bind each mutable projection to its immutable preparation result and
    /// retained slot. Historical results may precede later dispatch or retry.
    fn validate_progress(&self, state: &Snapshot, historical: bool) -> Result<()> {
        let fail = || CampaignError::Integrity("campaign progress/receipt/slot differs".into());
        // The accepted encoded-state capacity bounds iteration; no extra
        // reader-only cardinality rule may reject a committed public write.
        let slots: Vec<SlotHistory> = state
            .progress
            .attempts
            .iter()
            .map(|attempt| {
                self.slot_for_operation(&state.identity, &attempt.preparation_operation_id)?
                    .ok_or_else(fail)
            })
            .collect::<Result<_>>()?;
        let latest_by_obligation: BTreeMap<&str, &str> = state
            .progress
            .attempts
            .iter()
            .zip(&slots)
            .map(|(attempt, slot)| {
                (
                    slot.obligation_id.as_str(),
                    attempt.preparation_operation_id.as_str(),
                )
            })
            .collect();
        let mut attempts = BTreeSet::new();
        let mut operations = BTreeSet::new();
        let mut prior_by_obligation: BTreeMap<&str, (&str, bool)> = BTreeMap::new();
        for (index, attempt) in state.progress.attempts.iter().enumerate() {
            let slot = &slots[index];
            let prior = prior_by_obligation
                .get(slot.obligation_id.as_str())
                .copied();
            if !attempts.insert((slot.obligation_id.as_str(), attempt.attempt_id.as_str()))
                || !operations.insert(&attempt.preparation_operation_id)
                || attempt.predecessor.as_deref() != prior.map(|(id, _)| id)
                || (prior.is_none() && attempt.attempt_id != state.identity.attempt_id)
                || prior.is_some_and(|(_, failed)| !failed)
            {
                return Err(fail());
            }
            prior_by_obligation.insert(
                slot.obligation_id.as_str(),
                (
                    &attempt.attempt_id,
                    matches!(
                        attempt.outcome,
                        Some(crate::completion::AttemptOutcome::DefinitePreEntryFailure)
                    ),
                ),
            );
            let raw: Option<(String, String, String, String)> = self.conn.query_row(
                "SELECT kind,request_digest,receipt_json,result_json FROM operation_receipt WHERE operation_id=?1 AND identity_key=?2",
                params![attempt.preparation_operation_id, state.identity.key()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            ).optional().map_err(|e| CampaignError::Integrity(e.to_string()))?;
            let (kind, operation_digest, raw_receipt, raw_state) = raw.ok_or_else(fail)?;
            let expected_kind = if prior.is_none() {
                "prepare_initial"
            } else {
                "prepare_retry"
            };
            let receipt: OperationReceipt =
                serde_json::from_str(&raw_receipt).map_err(|_| fail())?;
            let prepared = self.decode_state(&raw_state, &state.identity)?;
            let obligation = prepared
                .obligations
                .iter()
                .find(|item| {
                    item.request.as_ref().is_some_and(|request| {
                        request.operation_id == attempt.preparation_operation_id
                    })
                })
                .ok_or_else(fail)?;
            let request = obligation.request.as_ref().ok_or_else(fail)?;
            if kind != expected_kind
                || receipt.kind != expected_kind
                || receipt.operation_id != attempt.preparation_operation_id
                || receipt.request_digest != operation_digest
                || receipt.root_id != self.root_id
                || receipt.identity != state.identity
                || receipt.profile_digest != self.marker.trusted_config_digest
                || receipt.result_revision != prepared.revision
                || receipt.result_reference
                    != format!(
                        "campaign:{}@{}",
                        state.identity.key(),
                        prepared.revision.as_str()
                    )
                || prepared.progress.attempts.get(index).is_none_or(|a| {
                    a.attempt_id != attempt.attempt_id
                        || a.request_digest != attempt.request_digest
                        || a.preparation_operation_id != attempt.preparation_operation_id
                })
                || request.kind != if prior.is_none() { "initial" } else { "retry" }
                || request.root_id != self.root_id
                || request.identity != state.identity
                || request.prepared_revision != prepared.revision
                || request.request_digest != attempt.request_digest
                || request.obligation_id != obligation.obligation_id
                || slot.obligation_id != obligation.obligation_id
                || slot.attempt_id != attempt.attempt_id
                || slot.request_digest != request.request_digest
                || slot.prepared_revision != request.prepared_revision.as_str()
                || (!historical
                    && slot.active
                        != (latest_by_obligation
                            .get(slot.obligation_id.as_str())
                            .copied()
                            == Some(attempt.preparation_operation_id.as_str())))
            {
                return Err(fail());
            }
            let expected_digest = if prior.is_none() {
                campaign_digest(
                    &json!({"rootId":request.root_id,"identity":request.identity,"obligationId":request.obligation_id,"operationId":request.operation_id,"kind":request.kind,"selection":request.selection,"candidate":request.candidate,"profileDigest":request.profile_digest}),
                )?
            } else {
                campaign_digest(
                    &json!({"rootId":request.root_id,"identity":request.identity,"obligationId":request.obligation_id,"operationId":request.operation_id,"kind":request.kind,"attemptId":attempt.attempt_id,"predecessor":attempt.predecessor,"selection":request.selection,"candidate":request.candidate,"profileDigest":request.profile_digest}),
                )?
            };
            if expected_digest != request.request_digest {
                return Err(fail());
            }
            if let Some(dispatch) = &attempt.dispatch {
                if !dispatch.may_have_entered
                    || dispatch.request_digest != attempt.request_digest
                    || slot.dispatch_operation_id.as_deref() != Some(dispatch.operation_id.as_str())
                    || !slot.may_have_entered
                    || !operations.insert(&dispatch.operation_id)
                {
                    return Err(fail());
                }
                let raw: Option<(String, String, String, String)> = self.conn.query_row(
                    "SELECT kind,request_digest,receipt_json,result_json FROM operation_receipt WHERE operation_id=?1 AND identity_key=?2",
                    params![dispatch.operation_id, state.identity.key()],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                ).optional().map_err(|e| CampaignError::Integrity(e.to_string()))?;
                let (kind, operation_digest, raw_receipt, raw_state) = raw.ok_or_else(fail)?;
                let receipt: OperationReceipt =
                    serde_json::from_str(&raw_receipt).map_err(|_| fail())?;
                let dispatched = self.decode_state(&raw_state, &state.identity)?;
                let recorded = dispatched
                    .progress
                    .attempts
                    .get(index)
                    .and_then(|a| a.dispatch.as_ref())
                    .ok_or_else(fail)?;
                let expected = campaign_digest(&json!({"identity":state.identity,
                    "obligationId":request.obligation_id,"preparationOperationId":request.operation_id,
                    "requestDigest":request.request_digest,"expectedRevision":receipt.prior_revision,
                    "operationId":dispatch.operation_id,"command":dispatch.command}))?;
                if kind != "authorize_dispatch"
                    || receipt.kind != kind
                    || receipt.operation_id != dispatch.operation_id
                    || receipt.root_id != self.root_id
                    || receipt.identity != state.identity
                    || receipt.profile_digest != self.marker.trusted_config_digest
                    || receipt.prior_revision.is_none()
                    || receipt.result_revision != dispatched.revision
                    || receipt.result_reference
                        != format!(
                            "campaign:{}@{}",
                            state.identity.key(),
                            dispatched.revision.as_str()
                        )
                    || receipt.request_digest != operation_digest
                    || operation_digest != expected
                    || recorded.operation_id != dispatch.operation_id
                    || recorded.request_digest != dispatch.request_digest
                    || recorded.command != dispatch.command
                    || !recorded.may_have_entered
                {
                    return Err(fail());
                }
            } else if !historical && slot.dispatch_operation_id.is_some() {
                return Err(fail());
            }
            if attempt.completion_intent.is_some() && attempt.dispatch.is_none() {
                return Err(fail());
            }
            if let Some(intent) = &attempt.completion_intent {
                let dispatch = attempt.dispatch.as_ref().ok_or_else(fail)?;
                if intent.episode_id != dispatch.command.episode_id
                    || intent.attempt_id != attempt.attempt_id
                    || intent.children.is_empty()
                    || intent.initial_episode_revision.is_some() != intent.result_digest.is_some()
                    || intent.result_digest.as_ref().is_some_and(|v| !valid_sha(v))
                {
                    return Err(fail());
                }
                let mut children = BTreeSet::new();
                for child in &intent.children {
                    if child.operation_id.trim().is_empty()
                        || !children.insert(&child.operation_id)
                        || !valid_sha(&child.content_digest)
                        || child.owner_root_id.trim().is_empty()
                        || child.grant_identity.trim().is_empty()
                    {
                        return Err(fail());
                    }
                }
                if !operations.insert(&intent.campaign_operation_id) {
                    return Err(fail());
                }
            }
            if matches!(
                attempt.outcome,
                Some(crate::completion::AttemptOutcome::DefinitePreEntryFailure)
            ) != attempt.failure_custody.is_some()
                || (attempt.outcome.is_some() && attempt.dispatch.is_none())
                || (attempt.failure_custody.is_some() && attempt.completion_intent.is_some())
            {
                return Err(fail());
            }
            if let Some(custody) = &attempt.failure_custody
                && (custody.reference.trim().is_empty() || !valid_sha(&custody.sha256))
            {
                return Err(fail());
            }
            if matches!(
                attempt.outcome,
                Some(
                    crate::completion::AttemptOutcome::Completed
                        | crate::completion::AttemptOutcome::FailedWithResult
                )
            ) && attempt
                .completion_intent
                .as_ref()
                .is_none_or(|intent| intent.result_digest.is_none())
            {
                return Err(fail());
            }
        }
        for obligation in &state.obligations {
            if let Some(request) = &obligation.request {
                let attempt = state
                    .progress
                    .attempts
                    .iter()
                    .find(|a| a.preparation_operation_id == request.operation_id)
                    .ok_or_else(fail)?;
                let slot = self
                    .slot_for_operation(&state.identity, &request.operation_id)?
                    .ok_or_else(fail)?;
                if request.identity != state.identity
                    || request.root_id != self.root_id
                    || request.obligation_id != obligation.obligation_id
                    || request.request_digest != attempt.request_digest
                    || request.prepared_revision.as_str() != slot.prepared_revision
                    || request.selection != *state.selection_ref.as_ref().ok_or_else(fail)?
                    || request.candidate != *state.candidate.as_ref().ok_or_else(fail)?
                    || !slot.active && !historical
                    || match obligation.status.as_str() {
                        "executing" => attempt.outcome.is_some(),
                        "completed" => !matches!(
                            attempt.outcome,
                            Some(crate::completion::AttemptOutcome::Completed)
                        ),
                        "failed" => !matches!(
                            attempt.outcome,
                            Some(crate::completion::AttemptOutcome::FailedWithResult)
                        ),
                        _ => true,
                    }
                {
                    return Err(fail());
                }
            }
        }
        let mut findings = BTreeSet::new();
        for evaluation in &state.progress.evaluations {
            if !findings.insert((&evaluation.finding_id, &evaluation.finding_revision))
                || !matches!(
                    &evaluation.reliance.kind,
                    crate::completion::ChildKind::Reliance
                )
                || !valid_sha(&evaluation.reliance.content_digest)
                || evaluation.finding_id.trim().is_empty()
                || evaluation.finding_revision.trim().is_empty()
                || evaluation.consumer_tree.trim().is_empty()
                || evaluation.decision_scope.trim().is_empty()
                || evaluation.reliance.operation_id.trim().is_empty()
                || evaluation.reliance.owner_root_id.trim().is_empty()
                || evaluation.reliance.grant_identity.trim().is_empty()
                || !operations.insert(&evaluation.reliance.operation_id)
            {
                return Err(fail());
            }
        }
        Ok(())
    }

    pub(crate) fn write(
        &mut self,
        state: &Snapshot,
        prior: Option<&CampaignRevision>,
        operation: &OperationReceipt,
        reserve_workspace: bool,
        reserve_slot: Option<&str>,
    ) -> Result<()> {
        self.verify_binding()?;
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| CampaignError::Io(e.to_string()))?;
        let state_json =
            serde_json::to_string(state).map_err(|e| CampaignError::Contract(e.to_string()))?;
        let receipt_json =
            serde_json::to_string(operation).map_err(|e| CampaignError::Contract(e.to_string()))?;
        if state_json.len() > 1_048_576 || receipt_json.len() > 65_536 {
            return Err(CampaignError::Contract(
                "encoded result exceeds capacity".into(),
            ));
        }
        if let Some(prior) = prior {
            let changed = tx.execute("UPDATE campaign_state SET revision=?1,state_json=?2 WHERE identity_key=?3 AND revision=?4", params![state.revision.as_str(),state_json,state.identity.key(),prior.as_str()])
                .map_err(|e| CampaignError::Io(e.to_string()))?;
            if changed != 1 {
                return Err(CampaignError::Conflict("campaign revision changed".into()));
            }
        } else {
            tx.execute(
                "INSERT INTO campaign_state(identity_key,revision,state_json) VALUES(?1,?2,?3)",
                params![state.identity.key(), state.revision.as_str(), state_json],
            )
            .map_err(|e| CampaignError::Conflict(e.to_string()))?;
        }
        if reserve_workspace {
            tx.execute(
                "INSERT INTO workspace_reservation(workspace,identity_key) VALUES(?1,?2)",
                params![state.workspace, state.identity.key()],
            )
            .map_err(|e| CampaignError::Conflict(e.to_string()))?;
        }
        tx.execute("INSERT INTO operation_receipt(operation_id,identity_key,kind,request_digest,receipt_json,result_json) VALUES(?1,?2,?3,?4,?5,?6)", params![operation.operation_id,state.identity.key(),operation.kind,operation.request_digest,receipt_json,state_json])
            .map_err(|e| CampaignError::Conflict(e.to_string()))?;
        if let Some(obligation_id) = reserve_slot {
            let request = state
                .obligations
                .iter()
                .find(|item| item.obligation_id == obligation_id && item.status == "executing")
                .and_then(|item| item.request.as_ref())
                .ok_or_else(|| {
                    CampaignError::Integrity("prepared request absent for slot".into())
                })?;
            if request.operation_id != operation.operation_id
                || request.identity != state.identity
                || request.root_id != self.root_id
                || request.prepared_revision != state.revision
            {
                return Err(CampaignError::Integrity(
                    "prepared slot/request differs".into(),
                ));
            }
            let attempt = state
                .progress
                .attempts
                .iter()
                .find(|attempt| attempt.preparation_operation_id == request.operation_id)
                .ok_or_else(|| CampaignError::Integrity("prepared attempt absent".into()))?;
            if attempt.preparation_operation_id != request.operation_id
                || attempt.request_digest != request.request_digest
            {
                return Err(CampaignError::Integrity("prepared attempt differs".into()));
            }
            if operation.kind == "prepare_retry" {
                let predecessor = attempt
                    .predecessor
                    .as_ref()
                    .ok_or_else(|| CampaignError::Integrity("retry predecessor absent".into()))?;
                let changed = tx.execute(
                    "UPDATE request_slot SET active=0 WHERE identity_key=?1 AND obligation_id=?2 AND attempt_id=?3 AND active=1 AND dispatch_operation_id IS NOT NULL",
                    params![state.identity.key(),obligation_id,predecessor],
                ).map_err(|e| CampaignError::Io(e.to_string()))?;
                if changed != 1 {
                    return Err(CampaignError::Conflict(
                        "retry predecessor slot differs".into(),
                    ));
                }
            }
            tx.execute("INSERT INTO request_slot(identity_key,obligation_id,attempt_id,operation_id,request_digest,prepared_revision,active) VALUES(?1,?2,?3,?4,?5,?6,1)", params![state.identity.key(),obligation_id,attempt.attempt_id,request.operation_id,request.request_digest,request.prepared_revision.as_str()])
                .map_err(|e| CampaignError::Conflict(e.to_string()))?;
        }
        if operation.kind == "authorize_dispatch" {
            let attempt = state
                .progress
                .attempts
                .iter()
                .find(|attempt| {
                    attempt
                        .dispatch
                        .as_ref()
                        .is_some_and(|dispatch| dispatch.operation_id == operation.operation_id)
                })
                .ok_or_else(|| CampaignError::Integrity("dispatch attempt absent".into()))?;
            let dispatch = attempt
                .dispatch
                .as_ref()
                .ok_or_else(|| CampaignError::Integrity("dispatch record absent".into()))?;
            if dispatch.operation_id != operation.operation_id
                || !dispatch.may_have_entered
                || dispatch.request_digest != attempt.request_digest
            {
                return Err(CampaignError::Integrity("dispatch state differs".into()));
            }
            let changed = tx.execute(
                "UPDATE request_slot SET dispatch_operation_id=?1,may_have_entered=1 WHERE identity_key=?2 AND operation_id=?3 AND request_digest=?4 AND dispatch_operation_id IS NULL AND may_have_entered=0 AND active=1",
                params![dispatch.operation_id, state.identity.key(), attempt.preparation_operation_id, attempt.request_digest],
            ).map_err(|e| CampaignError::Io(e.to_string()))?;
            if changed != 1 {
                return Err(CampaignError::Conflict("dispatch slot differs".into()));
            }
        }
        crate::fault_cut("before_commit");
        crate::fault_cut(&format!("before_commit_{}", operation.kind));
        tx.commit().map_err(|e| CampaignError::Io(e.to_string()))?;
        crate::fault_cut("after_commit");
        crate::fault_cut(&format!("after_commit_{}", operation.kind));
        Ok(())
    }

    pub(crate) fn slot(
        &self,
        identity: &CampaignIdentity,
        obligation_id: &str,
    ) -> Result<Option<(String, String, String)>> {
        self.verify_binding()?;
        self.conn.query_row("SELECT operation_id,request_digest,prepared_revision FROM request_slot WHERE identity_key=?1 AND obligation_id=?2 AND active=1", params![identity.key(),obligation_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)))
            .optional().map_err(|e| CampaignError::Io(e.to_string()))
    }

    pub(crate) fn dispatch_slot(
        &self,
        identity: &CampaignIdentity,
        obligation_id: &str,
    ) -> Result<Option<(Option<String>, bool)>> {
        self.verify_binding()?;
        self.conn.query_row(
            "SELECT dispatch_operation_id,may_have_entered FROM request_slot WHERE identity_key=?1 AND obligation_id=?2 AND active=1",
            params![identity.key(), obligation_id],
            |row| Ok((row.get(0)?, row.get::<_, i64>(1)? == 1)),
        ).optional().map_err(|e| CampaignError::Io(e.to_string()))
    }

    pub(crate) fn slot_for_operation(
        &self,
        identity: &CampaignIdentity,
        operation_id: &str,
    ) -> Result<Option<SlotHistory>> {
        self.verify_binding()?;
        self.conn.query_row(
            "SELECT obligation_id,attempt_id,request_digest,prepared_revision,dispatch_operation_id,may_have_entered,active FROM request_slot WHERE identity_key=?1 AND operation_id=?2",
            params![identity.key(), operation_id],
            |row| Ok(SlotHistory { obligation_id: row.get(0)?, attempt_id: row.get(1)?, request_digest: row.get(2)?,
                prepared_revision: row.get(3)?, dispatch_operation_id: row.get(4)?,
                may_have_entered: row.get::<_, i64>(5)? == 1, active: row.get::<_, i64>(6)? == 1 }),
        ).optional().map_err(|e| CampaignError::Io(e.to_string()))
    }

    pub(crate) fn reconcile(&self, locator: &RecoveryLocator) -> Reconcile {
        if self.root != locator.anchored_root || self.root_id != locator.root_id {
            return Reconcile::Unresolved("root identity differs".into());
        }
        match self.receipt(&locator.operation_id) {
            Ok(Some(receipt))
                if receipt.identity == locator.identity
                    && receipt.kind == locator.kind
                    && receipt.request_digest == locator.request_digest
                    && receipt.profile_digest == locator.profile_digest
                    && receipt.prior_revision == locator.expected_revision =>
            {
                Reconcile::Committed(receipt)
            }
            Ok(Some(receipt)) => Reconcile::Conflicting(receipt),
            Ok(None) => Reconcile::Absent,
            Err(error) => Reconcile::Unresolved(error.to_string()),
        }
    }
}

fn configure(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA trusted_schema=OFF; PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA busy_timeout=5000;")
        .map_err(|e| CampaignError::Io(e.to_string()))?;
    Ok(())
}

fn valid_sha(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(crate) fn state_revision(state: &Snapshot) -> Result<CampaignRevision> {
    let mut value =
        serde_json::to_value(state).map_err(|e| CampaignError::Contract(e.to_string()))?;
    strip_revision(&mut value)?;
    let digest = if state.phase == "accepted" {
        campaign_digest(&value)?
    } else {
        campaign_update_revision_digest(&value)?
    };
    CampaignRevision::new(digest)
}

fn strip_revision(value: &mut serde_json::Value) -> Result<()> {
    value
        .as_object_mut()
        .ok_or_else(|| CampaignError::Integrity("state not object".into()))?
        .remove("revision");
    if let Some(obligations) = value
        .get_mut("obligations")
        .and_then(serde_json::Value::as_array_mut)
    {
        for obligation in obligations {
            if let Some(request) = obligation
                .get_mut("request")
                .and_then(serde_json::Value::as_object_mut)
            {
                request.remove("preparedRevision");
            }
        }
    }
    Ok(())
}

pub(crate) fn trusted_digest<T: Serialize>(value: &T) -> Result<String> {
    let bytes = serde_json::to_vec(value).map_err(|e| CampaignError::Contract(e.to_string()))?;
    Ok(raw_sha256(&bytes))
}
